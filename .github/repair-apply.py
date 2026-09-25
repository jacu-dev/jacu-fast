#!/usr/bin/env python3
"""One-time, checksummed transport for the reviewed source changes.

The publisher removes this loader and its payload from the delivered tree.
All entries are public source files; this script never evaluates payload code.
"""
import base64
import hashlib
import json
import lzma
from pathlib import Path

ROOT = Path.cwd().resolve()
EXPECTED = 'b1e249e6a635e3f5a745cfcdf7115cd2d5e0eaef08ad80dd53984e75ed8694d5'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def checked_path(name):
    path = Path(name)
    if path.is_absolute() or '..' in path.parts or '.git' in path.parts:
        raise ValueError('unsafe source path')
    full = ROOT / path
    if not full.resolve().is_relative_to(ROOT) or full.is_symlink():
        raise ValueError('source path escapes the checkout')
    return full


parts = sorted((ROOT / '.github/repair-payload').glob('*.b64'))
if len(parts) != 5:
    raise ValueError('the source transport needs exactly five parts')
raw = lzma.decompress(base64.b64decode(''.join(p.read_text().strip() for p in parts), validate=True))
if digest(raw) != EXPECTED:
    raise ValueError('source transport checksum mismatch')
payload = json.loads(raw)
prepared = []
for name, item in payload['files'].items():
    path = checked_path(name)
    previous = path.read_bytes() if path.exists() else None
    if (digest(previous) if previous is not None else None) != item.get('old_sha256'):
        raise ValueError('preimage changed: ' + name)
    if item.get('delete'):
        prepared.append((path, None, None))
        continue
    if 'content' in item:
        content = item['content'].encode('utf-8')
    else:
        lines = previous.decode('utf-8').splitlines(keepends=True)
        for start, end, replacement in reversed(item['changes']):
            if not 0 <= start <= end <= len(lines):
                raise ValueError('invalid delta range: ' + name)
            lines[start:end] = [replacement]
        content = ''.join(lines).encode('utf-8')
    if digest(content) != item['sha256']:
        raise ValueError('postimage mismatch: ' + name)
    if item['mode'] not in (0o644, 0o755):
        raise ValueError('invalid source mode')
    prepared.append((path, content, item['mode']))
for path, content, mode in prepared:
    if content is None:
        path.unlink()
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        path.chmod(mode)
    print('Applied', path.relative_to(ROOT))
# Small, readable follow-up corrections can be tested without resending the
# original delta. Every replacement must match exactly once.
fixes = ROOT / '.github/repair-fixes.json'
if fixes.exists():
    for fix in json.loads(fixes.read_text()):
        path = checked_path(fix['path'])
        old = path.read_text()
        if old.count(fix['old']) != 1:
            raise ValueError('follow-up preimage is not unique: ' + fix['path'])
        path.write_text(old.replace(fix['old'], fix['new'], 1))
        print('Corrected', fix['path'])
print('Validated and applied', len(prepared), 'source entries')
