#!/usr/bin/env python3
"""Real CLI install/discovery smoke; does not use a model or create a paid session."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='jacu-claude-install-') as temporary:
    env = dict(os.environ, CLAUDE_CONFIG_DIR=temporary, DISABLE_AUTOUPDATER='1',
               CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC='1')
    def run(*args):
        result = subprocess.run(['claude', *args], env=env, stdin=subprocess.DEVNULL,
                                capture_output=True, text=True, timeout=90)
        print(result.stdout)
        if result.returncode:
            raise RuntimeError(f'claude {args[0]} failed: {result.stderr}')
        return result.stdout
    run('--version')
    run('plugin', 'validate', str(ROOT), '--strict')
    run('plugin', 'validate', str(ROOT / '.claude-plugin/plugin.json'), '--strict')
    run('plugin', 'validate', str(ROOT / 'skills'), '--strict')
    run('plugin', 'marketplace', 'add', str(ROOT))
    run('plugin', 'install', 'jacu-fast@jacu-fast', '--scope', 'user')
    listing = run('plugin', 'list', '--json')
    items = json.loads(listing)
    matches = [item for item in items if item.get('id') == 'jacu-fast@jacu-fast']
    assert len(matches) == 1, 'the real host did not register exactly one installation'
    installed = matches[0]
    assert installed['version'] == '0.3.0' and installed['enabled'] and not installed.get('errors'), installed
    package = Path(installed['installPath']).resolve(strict=True)
    run('plugin', 'validate', str(package / '.claude-plugin/plugin.json'), '--strict')
    details = run('plugin', 'details', 'jacu-fast@jacu-fast')
    assert 'jacu-fast' in details
    diagnostic = subprocess.run([sys.executable, str(package / 'scripts/host.py'), 'doctor'],
                                env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30)
    assert diagnostic.returncode == 0, diagnostic.stderr
    print(diagnostic.stdout)
    assert json.loads(diagnostic.stdout)['mode'] == 'runtime', 'installed cache lost the compatible executable'
    execution = subprocess.run([str(package / 'bin/jacu'), 'capabilities'], env=env,
                               stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30)
    assert execution.returncode == 0, execution.stderr
    assert json.loads(execution.stdout)['binary_version'] == '0.3.0'
    print(execution.stdout)
    # This proves host installation/registration, not that an authenticated model invoked the skill.
    print(json.dumps({'host': 'claude-code', 'installation': 'verified', 'installed_runtime': 'executed',
                      'authenticated_skill_invocation': 'not_exercised'}))
