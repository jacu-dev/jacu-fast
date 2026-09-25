"""Claude hook protocol tests. These do not claim an authenticated model-session test."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('jacu_host', ROOT / 'scripts/host.py')
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)


class HookProtocol(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='jacu-hook-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.env = patch.dict(os.environ, {'JACU_HOME': str(self.root / 'home')})
        self.env.start()
        self.addCleanup(self.env.stop)
        self.event = {'session_id': 'host-123', 'cwd': str(self.repo), 'stop_hook_active': False}
        self.state_path = host.active_path('host-123')

    def binding(self):
        host.atomic_json(self.state_path, {'repo': str(self.repo), 'workspace': str(self.repo),
                         'session': 's123', 'binary_version': host.version(), 'blocked': 0})

    def result(self, complete=False):
        return subprocess.CompletedProcess([], 0 if complete else 3, json.dumps({
            'session_id': 's123', 'exit_code': 0 if complete else 3,
            'outcome': 'complete' if complete else 'incomplete', 'terminal': complete,
            'candidate': {'hash': 'same'}, 'pending': [] if complete else ['O2 needs a test']} ).encode(), b'')

    def test_session_start_explains_exact_namespace_and_modes(self):
        result = host.start(self.event)
        self.assertEqual(result['hookSpecificOutput']['hookEventName'], 'SessionStart')
        self.assertIn('jacu-fast:jacu-fast', result['hookSpecificOutput']['additionalContext'])

    def test_inactive_stop_never_invokes_runtime_or_git(self):
        with patch.object(host, 'runtime_path', side_effect=AssertionError('must remain inactive')):
            self.assertEqual(host.stop(self.event), {})

    def test_stop_blocks_missing_evidence_without_running_verify(self):
        self.binding()
        with patch.object(host, 'runtime_path', return_value=Path('/fake/jacu')), patch.object(host.subprocess, 'run', return_value=self.result()) as run:
            value = host.stop(self.event)
        self.assertEqual(value['decision'], 'block')
        self.assertIn('O2', value['reason'])
        self.assertIn('report', run.call_args.args[0])
        self.assertNotIn('verify', run.call_args.args[0])

    def test_unchanged_stop_recursion_ends_honestly_without_loop(self):
        self.binding()
        with patch.object(host, 'runtime_path', return_value=Path('/fake/jacu')), patch.object(host.subprocess, 'run', return_value=self.result()):
            self.assertEqual(host.stop(self.event)['decision'], 'block')
            self.event['stop_hook_active'] = True
            value = host.stop(self.event)
        self.assertNotIn('decision', value)
        self.assertIn('INCOMPLETE', value['systemMessage'])
        self.assertFalse(self.state_path.exists())

    def test_a_completed_runtime_report_releases_binding(self):
        self.binding()
        with patch.object(host, 'runtime_path', return_value=Path('/fake/jacu')), patch.object(host.subprocess, 'run', return_value=self.result(True)):
            self.assertEqual(host.stop(self.event), {})
        self.assertFalse(self.state_path.exists())

    def test_other_workspace_does_not_inherit_gate(self):
        self.binding()
        self.event['cwd'] = str(self.root)
        with patch.object(host, 'runtime_path', side_effect=AssertionError('wrong workspace')):
            self.assertEqual(host.stop(self.event), {})

    def test_runtime_unavailable_is_not_reported_as_complete(self):
        self.binding()
        with patch.object(host, 'runtime_path', side_effect=ValueError('stale binary')):
            value = host.stop(self.event)
        self.assertIn('unavailable', value['systemMessage'])
        self.assertNotIn('decision', value)

    def test_request_cannot_supply_shell_commands_through_ids(self):
        with self.assertRaises(ValueError):
            host.active_path('../../other')

    def test_malformed_or_oversized_hook_stdin_is_bounded(self):
        run = subprocess.run(['python3', str(ROOT / 'scripts/host.py'), 'stop'], input=b'x' * 70000,
                             capture_output=True, timeout=5)
        self.assertEqual(run.returncode, 0)
        self.assertIn('unavailable', json.loads(run.stdout)['systemMessage'])

    def test_launcher_rejects_incompatible_explicit_runtime(self):
        fake = self.root / 'old-jacu'
        fake.write_text('#!/bin/sh\nprintf \'{"binary_version":"0.2.0"}\'\n')
        fake.chmod(0o755)
        with patch.dict(os.environ, {'JACU_BIN': str(fake)}):
            with self.assertRaisesRegex(ValueError, 'incompatible'):
                host.runtime_path()

    def test_doctor_distinguishes_disk_files_from_loaded_host_session(self):
        with patch.object(host, 'runtime_path', side_effect=ValueError('not packaged yet')):
            result = host.doctor()
        self.assertEqual(result['mode'], 'skill-only')
        self.assertIn('not observable', result['host_session_loaded'])

    def test_packaged_runtime_checksum_is_checked_before_execution(self):
        package = self.root / 'package'
        (package / '.claude-plugin').mkdir(parents=True)
        (package / '.claude-plugin/plugin.json').write_text('{"version":"0.3.0"}')
        (package / 'src').mkdir()
        for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
            (package / name).write_text('synthetic')
        (package / 'runtimes').mkdir()
        binary = package / 'runtimes' / 'jacu'
        binary.write_text('not an executable')
        (package / 'runtimes/manifest.json').write_text(json.dumps({'binary_version': '0.3.0',
             'source_sha256': host.source_hash(package), 'builds': {'x86_64-unknown-linux-gnu': {
             'path': 'runtimes/jacu', 'sha256': 'wrong'}}}))
        with patch.dict(os.environ, {}, clear=True), patch.object(host.platform, 'system', return_value='Linux'), patch.object(host.platform, 'machine', return_value='x86_64'):
            with self.assertRaisesRegex(ValueError, 'checksum'):
                host.runtime_path(package)


if __name__ == '__main__':
    unittest.main()
