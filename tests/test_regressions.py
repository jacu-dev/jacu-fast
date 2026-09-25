"""Black-box regressions against the compiled executable, not a model of its logic."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest


class RuntimeRegression(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        configured = os.environ.get('JACU_TEST_BIN')
        if not configured or not Path(configured).is_file():
            raise RuntimeError('JACU_TEST_BIN must name the actual freshly compiled jacu executable')
        cls.binary = str(Path(configured).resolve())

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='jacu-regression-')
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.repo = self.base / 'repo'
        self.repo.mkdir()
        self.env = dict(os.environ, JACU_HOME=str(self.base / 'home'), GIT_TERMINAL_PROMPT='0',
                        GIT_AUTHOR_NAME='Jacu Test', GIT_COMMITTER_NAME='Jacu Test',
                        GIT_AUTHOR_EMAIL='jacu@example.invalid', GIT_COMMITTER_EMAIL='jacu@example.invalid')
        self.git('init', '-b', 'main')
        (self.repo / 'value').write_text('original\n')
        self.git('add', 'value')
        self.git('commit', '-m', 'fixture')
        self.request = self.base / 'request.txt'
        self.request.write_text('The value must be good. Preserve this complete request.\n')
        self.contract = self.base / 'contract.json'
        self.script = self.base / 'check'
        self.counter = self.base / 'counter'
        self.script.write_text('#!/bin/sh\nprintf x >> "$1"\ngrep -qx good value\n')
        self.script.chmod(0o755)
        self.checks = [{'id': 'behavior', 'kind': 'external', 'program': str(self.script),
                        'args': [str(self.counter)], 'cwd': '.', 'timeout_seconds': 3}]
        self.write_policy()
        self.outcomes = [{'id': 'O1', 'statement': 'The value must be good.',
                          'source_quote': 'The value must be good.',
                          'implementation': 'present', 'delivery': 'in_target', 'evidence': ['behavior']}]
        self.write_contract()
        (self.repo / 'value').write_text('good\n')

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repo), *args], env=self.env,
                              capture_output=True, check=True).stdout

    def write_policy(self, required=None):
        (self.repo / '.jacu-fast.json').write_text(json.dumps({'schema_version': 1, 'checks': self.checks,
             'delivery': {'required_check_ids': required if required is not None else [c['id'] for c in self.checks]}}))

    def write_contract(self, **extra):
        body = {'schema_version': 1, 'outcomes': self.outcomes,
                'effort': {'uncertainty': 0, 'novelty': 0, 'coupling': 0, 'consequence': 'ordinary'}, **extra}
        self.contract.write_text(json.dumps(body))

    def run_cli(self, *args, env=None):
        run = subprocess.run([self.binary, *map(str, args)], env=env or self.env,
                             capture_output=True, stdin=subprocess.DEVNULL, timeout=35)
        result = json.loads(run.stdout)
        self.assertEqual(result['exit_code'], run.returncode, (result, run.stderr))
        return run.returncode, result

    def prepare(self, session=None):
        args = ['prepare', '--repo', self.repo, '--request-file', self.request, '--contract-file', self.contract]
        if session:
            args.extend(['--session', session])
        return self.run_cli(*args)

    def begin(self):
        code, result = self.prepare()
        self.assertEqual(code, 0, result)
        return result['session_id']

    def verify(self, session, retry=False):
        args = ['verify', '--repo', self.repo, '--session', session, '--checkpoint', 'delivery']
        if retry:
            args.append('--retry')
        return self.run_cli(*args)

    def report(self, session):
        return self.run_cli('report', '--repo', self.repo, '--session', session)

    def test_dirty_content_changes_invalidate_a_success_and_repair_a_failure(self):
        session = self.begin()
        code, good = self.verify(session)
        self.assertEqual(code, 0, good)
        (self.repo / 'value').write_text('bad\n')  # The porcelain status remains M.
        code, bad = self.verify(session)
        self.assertEqual(code, 2, bad)
        self.assertNotEqual(good['candidate']['hash'], bad['candidate']['hash'])
        (self.repo / 'value').write_text('good\ngood\n')
        code, repaired = self.verify(session)
        self.assertEqual(code, 0, repaired)
        self.assertEqual(self.counter.read_text(), 'xxx')

    def test_existing_untracked_content_is_not_hidden_by_the_same_status(self):
        self.git('rm', '--cached', 'value')
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        (self.repo / 'value').write_text('bad\n')
        self.assertEqual(self.verify(session)[0], 2)
        self.assertEqual(self.counter.read_text(), 'xx')

    def test_nongit_content_changes_are_observed(self):
        shutil.rmtree(self.repo / '.git')
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        (self.repo / 'value').write_text('bad\n')
        self.assertEqual(self.verify(session)[0], 2)

    def test_same_candidate_reuses_a_success_without_rerunning(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.report(session)[0], 0)
        self.assertEqual(self.counter.read_text(), 'x')

    def test_changed_program_outside_repository_invalidates_evidence(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        self.script.write_text('#!/bin/sh\nexit 1\n')
        self.assertEqual(self.verify(session)[0], 2)

    def test_effective_environment_change_invalidates_evidence(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        self.env['JACU_TEST_INPUT'] = 'changed'
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.counter.read_text(), 'xx')

    def test_shell_bookkeeping_is_removed_from_environment_and_fingerprint(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        self.env['_'] = '/different/launcher'
        self.env['SHLVL'] = '18'
        self.assertEqual(self.report(session)[0], 0)
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.counter.read_text(), 'x')

    def test_explicit_ignored_inputs_invalidate_receipts(self):
        (self.repo / '.gitignore').write_text('fixture\n')
        (self.repo / 'fixture').write_text('one\n')
        self.checks[0]['inputs'] = ['fixture']
        self.write_policy()
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        (self.repo / 'fixture').write_text('two\n')
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.counter.read_text(), 'xx')

    def test_changed_file_mode_invalidates_receipts(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        (self.repo / 'value').chmod(0o755)
        self.assertEqual(self.verify(session)[0], 0)
        self.assertEqual(self.counter.read_text(), 'xx')

    def test_same_dirty_file_mutated_during_check_cannot_pass(self):
        self.script.write_text('#!/bin/sh\nprintf changed >> value\nexit 0\n')
        session = self.begin()
        code, result = self.verify(session)
        self.assertEqual(code, 3, result)
        self.assertNotEqual(result['outcome'], 'complete')

    def test_policy_command_rewrite_is_rejected_before_execution(self):
        session = self.begin()
        self.checks[0]['program'] = shutil.which('true')
        self.assertIsNotNone(self.checks[0]['program'], 'the fixture needs a real true executable')
        self.checks[0]['args'] = []
        self.write_policy()
        code, result = self.verify(session)
        self.assertEqual(code, 3, result)
        self.assertFalse(self.counter.exists())

    def test_removing_required_check_cannot_weaken_delivery(self):
        session = self.begin()
        self.write_policy(required=[])
        self.assertEqual(self.verify(session)[0], 3)

    def test_request_cannot_be_rebound_in_the_same_session(self):
        session = self.begin()
        self.request.write_text('A convenient different task.\n')
        self.assertEqual(self.prepare(session)[0], 4)

    def test_outcome_statement_cannot_be_rewritten(self):
        session = self.begin()
        self.outcomes[0]['statement'] = 'Anything is good.'
        self.write_contract()
        self.assertEqual(self.prepare(session)[0], 4)

    def test_self_approved_scope_amendment_is_rejected(self):
        session = self.begin()
        self.write_contract(amendments=[{'id': 'A1', 'removes': ['O1'], 'reason': 'faster', 'source': 'model'}])
        self.assertEqual(self.prepare(session)[0], 4)

    def test_source_quote_must_exist_in_original_request(self):
        self.outcomes[0]['source_quote'] = 'An invented requirement.'
        self.write_contract()
        self.assertEqual(self.prepare()[0], 4)

    def test_switching_branch_does_not_deliver_to_the_pinned_target(self):
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 0)
        self.git('switch', '-c', 'elsewhere')
        code, result = self.report(session)
        self.assertEqual(code, 3, result)
        self.assertEqual(result['delivery_target']['branch'], 'main')

    def test_absent_tests_are_not_a_passing_behavioral_check(self):
        self.checks[0]['kind'] = 'cargo-test'
        self.write_policy()
        self.script.write_text('#!/bin/sh\nprintf "running 0 tests\\n"\nexit 0\n')
        self.assertEqual(self.verify(self.begin())[0], 2)

    def test_nonempty_cargo_suite_plus_empty_doctests_is_valid(self):
        self.checks[0]['kind'] = 'cargo-test'
        self.write_policy()
        self.script.write_text('#!/bin/sh\nprintf "test result: ok. 3 passed; 0 failed\\nrunning 0 tests\\ntest result: ok. 0 passed; 0 failed\\n"\n')
        self.assertEqual(self.verify(self.begin())[0], 0)

    def test_unknown_test_output_is_unknown_not_a_pass(self):
        self.checks[0]['kind'] = 'test'
        self.write_policy()
        self.script.write_text('#!/bin/sh\nprintf "some text\\n"\n')
        code, result = self.verify(self.begin())
        self.assertEqual(code, 3, result)
        self.assertEqual(result['checks'][0]['state'], 'unknown')

    def test_semantic_zero_cannot_approve_and_high_score_is_not_sufficient_alone(self):
        self.checks[0]['kind'] = 'semantic'
        self.write_policy()
        answer = {'request_sha256': hashlib.sha256(self.request.read_bytes()).hexdigest(), 'evidence_id': 'behavior', 'score': 0}
        self.script.write_text("#!/bin/sh\nprintf '%s\\n' '" + json.dumps(answer) + "'\n")
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 2)
        answer['score'] = 1
        self.script.write_text("#!/bin/sh\nprintf '%s\\n' '" + json.dumps(answer) + "'\n")
        code, result = self.verify(session)
        self.assertEqual(code, 3, result)
        self.assertEqual(result['checks'][0]['state'], 'passed')
        self.assertNotEqual(result['outcome'], 'complete')

    def test_retry_after_diagnosed_external_recovery_does_not_require_dummy_edit(self):
        state = self.base / 'service-state'
        state.write_text('down')
        self.script.write_text('#!/bin/sh\ngrep -qx up "' + str(state) + '"\n')
        session = self.begin()
        self.assertEqual(self.verify(session)[0], 2)
        state.write_text('up\n')
        self.assertEqual(self.verify(session)[0], 2)
        self.assertEqual(self.verify(session, retry=True)[0], 0)

    def test_background_descendant_with_open_pipe_is_bounded(self):
        self.checks[0]['timeout_seconds'] = 1
        self.write_policy()
        self.script.write_text('#!/bin/sh\nsleep 20 &\nexit 0\n')
        session = self.begin()
        start = time.monotonic()
        code, result = self.verify(session)
        self.assertEqual(code, 3, result)
        self.assertLess(time.monotonic() - start, 6)

    def test_existing_node_scripts_are_discovered_without_project_configuration(self):
        (self.repo / '.jacu-fast.json').unlink()
        (self.repo / 'package.json').write_text(json.dumps({'scripts': {'typecheck': 'tsc --noEmit', 'test': 'vitest run'}}))
        code, result = self.run_cli('prepare', '--repo', self.repo, '--request-file', self.request)
        self.assertEqual(code, 0, result)
        self.assertIn('discovered', result['policy_source'])
        self.assertEqual([x['id'] for x in result['discovered_checks']], ['node-typecheck', 'node-test'])
        self.assertFalse((self.repo / '.jacu-fast.json').exists())

    def test_old_session_version_cannot_be_silently_migrated(self):
        session = self.begin()
        state = next((self.base / 'home' / 'repos').rglob(session + '.json'))
        body = json.loads(state.read_text())
        body['binary_version'] = '0.2.0'
        state.write_text(json.dumps(body))
        self.assertEqual(self.prepare(session)[0], 4)
        self.assertEqual(self.report(session)[0], 4)

    def test_symlink_input_cannot_escape_repository(self):
        (self.repo / 'link').symlink_to(self.request)
        self.assertEqual(self.prepare()[0], 4)


if __name__ == '__main__':
    unittest.main()
