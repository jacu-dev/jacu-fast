"""Real launcher -> runtime -> completion-hook integration; no mocked receipts."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class RealHookWorkflow(unittest.TestCase):
    def test_launcher_and_hook_agree_on_effective_working_directory(self):
        binary = Path(os.environ['JACU_TEST_BIN']).resolve(strict=True)
        with tempfile.TemporaryDirectory(prefix='jacu-real-hook-') as directory:
            base = Path(directory).resolve()
            repo = base / 'repo'
            repo.mkdir()
            env = dict(os.environ, JACU_HOME=str(base / 'state'), JACU_BIN=str(binary),
                       PWD=str(base / 'stale-parent-directory'),
                       GIT_AUTHOR_NAME='Jacu Test', GIT_AUTHOR_EMAIL='jacu@example.invalid',
                       GIT_COMMITTER_NAME='Jacu Test', GIT_COMMITTER_EMAIL='jacu@example.invalid')
            launcher = ROOT / 'bin/jacu'
            adapter = ROOT / 'scripts/host.py'

            def run(argv, data=None, code=0):
                result = subprocess.run([str(x) for x in argv], cwd=repo, env=env,
                                        input=data, text=True, capture_output=True, timeout=35)
                self.assertEqual(result.returncode, code, (result.stdout, result.stderr))
                return json.loads(result.stdout) if result.stdout.lstrip().startswith('{') else result.stdout

            run(['git', 'init', '-b', 'main'])
            (repo / 'feature.txt').write_text('implemented\n')
            count = base / 'count'
            # Python does not repair PWD as a shell would. Verify the actual child
            # receives its canonical execution directory, then observe reuse.
            script = base / 'check.py'
            script.write_text("import os\nfrom pathlib import Path\n"
                              "assert Path(os.environ['PWD']) == Path.cwd().resolve()\n"
                              "assert Path('feature.txt').read_text() == 'implemented\\n'\n"
                              "with Path(__import__('sys').argv[1]).open('a') as f: f.write('x')\n")
            policy = {'schema_version': 1, 'checks': [
                {'id': 'smoke', 'program': sys.executable, 'args': [str(script), str(count)],
                 'timeout_seconds': 5, 'reuse': 'same-session-declared-inputs'}],
                'delivery': {'required_check_ids': ['smoke']}}
            (repo / '.jacu-fast.json').write_text(json.dumps(policy))
            run(['git', 'add', '.'])
            run(['git', 'commit', '-m', 'synthetic fixture'])
            request = base / 'request.txt'
            request.write_text('Deliver the feature.\n')
            contract = base / 'contract.json'
            contract.write_text(json.dumps({'schema_version': 1, 'outcomes': [
                {'id': 'O1', 'statement': 'Deliver the feature.', 'source_quote': 'Deliver the feature.',
                 'implementation': 'present', 'delivery': 'in_target', 'evidence': ['smoke']}],
                'effort': {'uncertainty': 0, 'novelty': 0, 'coupling': 0, 'consequence': 'ordinary'}}))
            prepared = run([launcher, 'prepare', '--repo', repo, '--request-file', request,
                            '--contract-file', contract])
            session = prepared['session_id']
            bound = run([sys.executable, adapter, 'activate', '--repo', repo, '--session', session,
                         '--host-session', 'real-integration-fixture'])
            self.assertTrue(bound['hook_bound'])
            event = json.dumps({'session_id': 'real-integration-fixture', 'cwd': str(repo),
                                'stop_hook_active': False})
            before = run([sys.executable, adapter, 'stop'], event)
            self.assertEqual(before.get('decision'), 'block', before)
            verified = run([launcher, 'verify', '--repo', repo, '--session', session,
                            '--checkpoint', 'delivery'])
            self.assertEqual(verified['outcome'], 'complete', verified)
            env['PWD'] = str(base / 'a-different-stale-directory')
            direct = run([binary, 'verify', '--repo', repo, '--session', session,
                          '--checkpoint', 'delivery'])
            self.assertEqual(direct['outcome'], 'complete', direct)
            self.assertEqual(count.read_text(), 'x', 'unchanged effective inputs should reuse evidence')
            after = run([sys.executable, adapter, 'stop'], event)
            self.assertEqual(after, {}, after)
            self.assertEqual(count.read_text(), 'x', 'completion hooks must not execute checks')
            self.assertFalse(list((base / 'state/hosts/claude').glob('*.json')))


if __name__ == '__main__':
    unittest.main()
