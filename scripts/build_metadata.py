#!/usr/bin/env python3
"""Record the source and executable fingerprint after successful test/build steps."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('jacu_host', ROOT / 'scripts/host.py')
host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(host)
target = {('Darwin', 'arm64'): 'aarch64-apple-darwin',
          ('Linux', 'x86_64'): 'x86_64-unknown-linux-gnu'}[(platform.system(), platform.machine())]
output = ROOT / 'artifacts'
output.mkdir(exist_ok=True)
binary = ROOT / 'target/release/jacu'
subprocess.run([str(binary), 'capabilities'], check=True)
shutil.copyfile(binary, output / 'jacu')
(output / 'jacu').chmod(0o755)
value = {'target': target, 'binary_version': host.version(), 'source_sha256': host.source_hash(ROOT),
         'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
         'compiler': subprocess.check_output(['rustc', '--version'], text=True).strip(),
         'ci_commit': os.environ.get('GITHUB_SHA', 'local'),
         'ci_run': os.environ.get('GITHUB_RUN_ID', 'local'),
         'platform': platform.platform()}
(output / 'build.json').write_text(json.dumps(value, indent=2) + '\n')
print(json.dumps(value))
