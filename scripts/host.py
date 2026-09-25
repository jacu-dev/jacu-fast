#!/usr/bin/env python3
"""Claude adapter and package diagnostics. No project checks run inside a hook."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import select
import subprocess
import sys
import tempfile
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
LIMIT = 65_536
REPORT_TIMEOUT = 22


def version(root: Path = ROOT) -> str:
    return json.loads((root / '.claude-plugin/plugin.json').read_text())['version']


def source_hash(root: Path) -> str:
    h = hashlib.sha256()
    paths = [root / n for n in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')]
    paths.extend(sorted((root / 'src').rglob('*.rs')))
    for path in sorted(paths):
        name = path.relative_to(root).as_posix().encode()
        body = path.read_bytes()
        h.update(len(name).to_bytes(8, 'little'))
        h.update(name)
        h.update(len(body).to_bytes(8, 'little'))
        h.update(body)
    return h.hexdigest()


def runtime_path(root: Path = ROOT) -> Path:
    override = os.environ.get('JACU_BIN')
    if override:
        path = Path(override)
        if not path.is_absolute() or not path.is_file():
            raise ValueError('JACU_BIN must be an absolute executable file path')
        if path.resolve() == (root / 'bin/jacu').resolve():
            raise ValueError('JACU_BIN must not point to the launcher itself')
    else:
        target = {('Darwin', 'arm64'): 'aarch64-apple-darwin',
                  ('Linux', 'x86_64'): 'x86_64-unknown-linux-gnu'}.get((platform.system(), platform.machine()))
        if target is None:
            raise ValueError('no packaged runtime for this platform; skill-only mode remains available')
        manifest_path = root / 'runtimes/manifest.json'
        if not manifest_path.is_file():
            raise ValueError('prebuilt runtimes are not present in this checkout; use skill-only mode or an explicit developer build')
        manifest = json.loads(manifest_path.read_text())
        if manifest['binary_version'] != version(root) or manifest['source_sha256'] != source_hash(root):
            raise ValueError('packaged runtime is stale for this version/source; refusing old evidence')
        item = manifest['builds'][target]
        path = (root / item['path']).resolve()
        if not path.is_relative_to((root / 'runtimes').resolve()) or not path.is_file():
            raise ValueError('invalid packaged runtime path')
        if hashlib.sha256(path.read_bytes()).hexdigest() != item['sha256']:
            raise ValueError('packaged runtime checksum mismatch')
    if not os.access(path, os.X_OK):
        raise ValueError('runtime exists but is not executable')
    info = subprocess.run([str(path), 'capabilities'], stdin=subprocess.DEVNULL,
                          capture_output=True, timeout=4, check=False)
    if info.returncode != 0 or len(info.stdout) > LIMIT:
        raise ValueError('runtime capability check failed')
    caps = json.loads(info.stdout)
    if caps.get('binary_version') != version(root) or caps.get('snapshot') != 'tracked_and_untracked_content_v2':
        raise ValueError('runtime version or evidence protocol is incompatible')
    return path


def home() -> Path:
    return Path(os.environ.get('JACU_HOME', Path.home() / '.jacu-fast'))


def active_path(session_id: str) -> Path:
    if not re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', session_id):
        raise ValueError('invalid host session id')
    return home() / 'hosts' / 'claude' / (hashlib.sha256(session_id.encode()).hexdigest() + '.json')


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd, name = tempfile.mkstemp(prefix='.write-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as f:
            json.dump(value, f, ensure_ascii=True)
            f.flush()
            os.fsync(f.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def read_event() -> dict[str, Any]:
    """Bound an open stdin pipe, and accept a complete object without waiting for EOF."""
    end = time.monotonic() + 2
    data = b''
    while time.monotonic() < end:
        ready, _, _ = select.select([sys.stdin.buffer], [], [], max(0, end - time.monotonic()))
        if not ready:
            break
        chunk = os.read(sys.stdin.fileno(), min(8192, LIMIT + 1 - len(data)))
        if not chunk:
            break
        data += chunk
        if len(data) > LIMIT:
            raise ValueError('hook input is too large')
        try:
            event = json.loads(data)
            if not isinstance(event, dict):
                raise ValueError('hook input must be an object')
            return event
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
    raise ValueError('hook input is missing, incomplete, or timed out')


def start(event: dict[str, Any]) -> dict[str, Any]:
    return {'hookSpecificOutput': {'hookEventName': 'SessionStart', 'additionalContext':
        'Jacu Fast is installed. When the user asks to use Jacu Fast, invoke the '
        'jacu-fast:jacu-fast skill; natural-language requests are supported. '
        'The short /jacu-fast command belongs to standalone skill installs. '
        'The skill remains usable without the optional runtime, but must report skill-only mode. '
        'Do not silently ignore an explicit Jacu request.'}}


def activate(repo: str, session: str, host_session: str, workspace: str | None = None) -> dict[str, Any]:
    if not re.fullmatch(r'[A-Za-z0-9_.-]{1,64}', session):
        raise ValueError('invalid Jacu session id')
    root = Path(repo).resolve(strict=True)
    binary = runtime_path()
    check = subprocess.run([str(binary), 'report', '--repo', str(root), '--session', session],
                           stdin=subprocess.DEVNULL, capture_output=True, timeout=REPORT_TIMEOUT)
    if check.returncode not in (0, 2, 3) or len(check.stdout) > 2_097_152:
        raise ValueError('cannot bind an unknown or invalid Jacu session')
    result = json.loads(check.stdout)
    if result.get('session_id') != session:
        raise ValueError('runtime returned a different session')
    atomic_json(active_path(host_session), {'repo': str(root), 'workspace': str(Path(workspace or os.getcwd()).resolve(strict=True)), 'session': session,
                'binary_version': version(), 'blocked': 0, 'last_signature': '', 'created_at': time.time()})
    return {'mode': 'runtime', 'host': 'claude', 'session_id': session, 'hook_bound': True}


def stop(event: dict[str, Any]) -> dict[str, Any]:
    host_id = event.get('session_id')
    if not isinstance(host_id, str):
        return {}
    path = active_path(host_id)
    if not path.is_file():
        return {}  # Inactive hook: no Git, binary, tests, network or model calls.
    if path.stat().st_size > LIMIT:
        return {'systemMessage': 'Jacu verification unavailable: invalid session binding. Do not claim verified completion.'}
    state = json.loads(path.read_text())
    cwd = Path(event.get('cwd', '')).resolve()
    root = Path(state['repo']).resolve()
    workspace = Path(state.get('workspace', state['repo'])).resolve()
    if not cwd.is_relative_to(workspace):
        return {}  # A different project must not inherit this task's gate.
    try:
        if state.get('binary_version') != version():
            raise ValueError('the active task belongs to a different Jacu version')
        binary = runtime_path()
        run = subprocess.run([str(binary), 'report', '--repo', str(root), '--session', state['session']],
                             stdin=subprocess.DEVNULL, capture_output=True, timeout=REPORT_TIMEOUT)
        if run.returncode not in (0, 2, 3) or len(run.stdout) > 2_097_152:
            raise ValueError('runtime report failed or exceeded its output limit')
        result = json.loads(run.stdout)
        if result.get('session_id') != state['session'] or result.get('exit_code') != run.returncode:
            raise ValueError('runtime report identity or exit status mismatch')
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        return {'systemMessage': f'Jacu verification unavailable: {exc}. The task is not certified complete; report the limitation.'}
    outcome = result.get('outcome')
    if outcome in ('complete', 'assessment_complete') and run.returncode == 0:
        path.unlink(missing_ok=True)
        return {}
    pending = result.get('pending', [])
    detail = '; '.join(str(x) for x in pending[:8])[:2500] or 'required evidence is incomplete'
    signature = hashlib.sha256(json.dumps({'outcome': outcome, 'pending': pending,
                              'candidate': result.get('candidate')}, sort_keys=True).encode()).hexdigest()
    repeated = signature == state.get('last_signature')
    # The host's recursion flag is honored. Unchanged work gets one recovery turn, not an endless loop.
    if result.get('terminal') or (event.get('stop_hook_active') and repeated) or state.get('blocked', 0) >= 3:
        path.unlink(missing_ok=True)
        return {'systemMessage': f'Jacu ended INCOMPLETE ({outcome}): {detail}. No approval is requested and no passing result was recorded.'}
    state['blocked'] = state.get('blocked', 0) + 1
    state['last_signature'] = signature
    atomic_json(path, state)
    return {'decision': 'block', 'reason':
            f'Jacu task {state["session"]} is not complete: {detail}. '
            'Continue in this session: repair the relevant behavior, obtain evidence with verify, '
            'and integrate into the recorded target. Do not ask for routine approval or weaken requirements. '
            'A real external blocker must be reported as incomplete.'}


def doctor() -> dict[str, Any]:
    result: dict[str, Any] = {'binary_version': version(), 'runtime_provenance': 'explicit developer override' if os.environ.get('JACU_BIN') else 'source-bound package manifest', 'skill_present': (ROOT / 'skills/jacu-fast/SKILL.md').is_file(),
             'plugin_command': '/jacu-fast:jacu-fast', 'standalone_command': '/jacu-fast',
             'host_session_loaded': 'not observable from package files',
             'hooks_registered_in_package': (ROOT / 'hooks/hooks.json').is_file()}
    try:
        result.update(mode='runtime', runtime=str(runtime_path()), runtime_compatible=True)
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        result.update(mode='skill-only', runtime_compatible=False, reason=str(exc))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest='command', required=True)
    for name in ('start', 'stop', 'doctor', 'exec'):
        p = sub.add_parser(name)
        if name == 'exec':
            p.add_argument('argv', nargs=argparse.REMAINDER)
    bind = sub.add_parser('activate')
    bind.add_argument('--repo', required=True)
    bind.add_argument('--session', required=True)
    bind.add_argument('--host-session', required=True)
    bind.add_argument('--workspace', help='Claude working directory when delivery uses another linked worktree')
    args = parser.parse_args()
    hook = args.command in ('start', 'stop')
    try:
        if args.command == 'exec':
            binary = runtime_path()
            argv = args.argv[1:] if args.argv[:1] == ['--'] else args.argv
            os.execv(str(binary), [str(binary), *argv])
        elif args.command == 'doctor':
            value = doctor()
        elif args.command == 'activate':
            value = activate(args.repo, args.session, args.host_session, args.workspace)
        else:
            event = read_event()
            value = start(event) if args.command == 'start' else stop(event)
        print(json.dumps(value))
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        if hook:
            print(json.dumps({'systemMessage': f'Jacu hook unavailable: {exc}. Do not claim automated verification.'}))
            return 0  # Host protocol, not the CLI's exit-code protocol.
        print(json.dumps({'outcome': 'unavailable', 'exit_code': 3, 'mode': 'skill-only', 'reason': str(exc)}))
        return 3


if __name__ == '__main__':
    raise SystemExit(main())
