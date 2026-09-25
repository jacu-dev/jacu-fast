#!/usr/bin/env python3
"""Assemble only binaries produced by matching, tested source artifacts."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('jacu_host', ROOT / 'scripts/host.py')
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT)
    args = parser.parse_args()
    expected = host.source_hash(args.output)
    builds = {}
    for metadata_file in sorted(args.artifacts.rglob('build.json')):
        item = json.loads(metadata_file.read_text())
        target = item['target']
        if target not in ('aarch64-apple-darwin', 'x86_64-unknown-linux-gnu') or target in builds:
            raise ValueError('unexpected or duplicate build target')
        if item['source_sha256'] != expected or item['binary_version'] != host.version(args.output):
            raise ValueError('source/version mismatch between the artifact and package')
        binary = metadata_file.parent / 'jacu'
        checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
        if checksum != item['sha256']:
            raise ValueError('artifact checksum mismatch')
        destination = args.output / 'runtimes' / target / 'jacu'
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(binary, destination)
        destination.chmod(0o755)
        builds[target] = {**item, 'path': destination.relative_to(args.output).as_posix()}
    if not builds:
        raise ValueError('no tested builds supplied')
    manifest = {'schema_version': 1, 'binary_version': host.version(args.output),
                'source_sha256': expected, 'builds': builds}
    (args.output / 'runtimes/manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'packaged_targets': list(builds), 'source_sha256': expected}))

if __name__ == '__main__':
    main()
