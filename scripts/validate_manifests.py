#!/usr/bin/env python3
"""Offline package invariants; authoritative Claude validation is a separate CI step."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

def main():
    plugin = json.loads((ROOT / '.claude-plugin/plugin.json').read_text())
    portable = json.loads((ROOT / 'plugin.json').read_text())
    market = json.loads((ROOT / '.claude-plugin/marketplace.json').read_text())
    entry = market['plugins'][0]
    version = plugin['version']
    assert version == portable['version'] == entry['version']
    assert plugin['name'] == portable['name'] == entry['name'] == 'jacu-fast'
    assert entry['source'] == './'
    cargo = (ROOT / 'Cargo.toml').read_text()
    assert re.search(r'^version = "' + re.escape(version) + r'"$', cargo, re.M)
    assert 'name = "jacu-fast"\nversion = "' + version + '"' in (ROOT / 'Cargo.lock').read_text()
    skill = (ROOT / 'skills/jacu-fast/SKILL.md').read_text()
    assert skill.startswith('---\n') and '\nname: jacu-fast\n' in skill
    assert 'disable-model-invocation: true' not in skill
    assert 'skill-only' in skill and '/jacu-fast:jacu-fast' in skill
    hooks = json.loads((ROOT / 'hooks/hooks.json').read_text())['hooks']
    assert set(hooks) == {'SessionStart', 'Stop'}
    for event, groups in hooks.items():
        for group in groups:
            for hook in group['hooks']:
                assert hook['type'] == 'command'
                assert '"${CLAUDE_PLUGIN_ROOT}/scripts/host.py"' in hook['command']
                assert hook['timeout'] <= 30
    for path in ('scripts/host.py', 'bin/jacu', 'skills/jacu-fast/references/runtime.md'):
        assert (ROOT / path).is_file(), path
    assert (ROOT / 'bin/jacu').read_text().startswith('#!/bin/sh\n')
    policy = json.loads((ROOT / 'examples/policy.json').read_text())
    assert set(policy) == {'schema_version', 'checks', 'delivery'}
    print('Offline manifests, skill, hook wiring and versions are consistent.')

if __name__ == '__main__':
    main()
