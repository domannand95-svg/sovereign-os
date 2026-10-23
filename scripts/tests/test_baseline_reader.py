"""Failure and framing contracts; no toolchain, network or provider needed."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from contextlib import contextmanager, redirect_stderr, redirect_stdout
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'verify-baseline-reader.py'
spec = importlib.util.spec_from_file_location('baseline_reader', SCRIPT)
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)
REPO = SCRIPT.parent.parent
FIXTURE = REPO / 'crates/beta001-harness/tests/fixtures/unicode17_v1_package.json'


class BaselineReaderTests(unittest.TestCase):
    @contextmanager
    def simulated_tools(self, root, *, tree=None, lock=None,
                        rustc='rustc 1.97.1 synthetic\n',
                        cargo='cargo 1.97.1 synthetic\n', failure=None):
        """Exercise real export/report/cleanup with synthetic external processes.

        This tar and fake executables are test machinery, never release evidence.
        No Git objects, compiler, Cargo cache or network are needed by these cases.
        """
        # Synthetic exported Git bytes, independent of checkout CRLF conversion.
        # Only the test's expected lock hash changes; runtime pins stay untouched.
        lock_bytes = b'synthetic baseline lock\n'
        sysroot = root / 'toolchain'
        (sysroot / 'bin').mkdir(parents=True)
        suffix = '.exe' if os.name == 'nt' else ''
        for name in ['rustc', 'cargo']:
            (sysroot / 'bin' / (name + suffix)).write_bytes(b'synthetic tool identity')
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode='w') as tar:
            for name, data in [('Cargo.lock', lock_bytes),
                               ('crates/beta001-harness/placeholder', b'')]:
                entry = tarfile.TarInfo(name)
                entry.size = len(data)
                tar.addfile(entry, io.BytesIO(data))
        state = {'builds': [], 'readers': [], 'commands': []}

        def command(args, **kwargs):
            state['commands'].append(args)
            if args[0] == 'git':
                if args[3] == 'rev-parse':
                    return ((tree if tree is not None else harness.TREE) + '\n').encode()
                if args[3] == 'show':
                    return lock_bytes if lock is None else lock
                if args[3] == 'archive':
                    return archive.getvalue()
            if args == ['rustc', '-vV']:
                return rustc.encode()
            if args == ['cargo', '-V']:
                return cargo.encode()
            if args == ['rustc', '--print', 'sysroot']:
                return (str(sysroot) + '\n').encode()
            if args[:2] == ['cargo', 'build']:
                self.assertIn('--offline', args)
                self.assertIn('--locked', args)
                self.assertEqual(kwargs['env']['CARGO_NET_OFFLINE'], 'true')
                source = Path(kwargs['cwd'])
                state['builds'].append(source.parent)
                if failure == 'second-build' and len(state['builds']) == 2:
                    raise RuntimeError('synthetic second build failed')
                if failure == 'build-timeout':
                    raise subprocess.TimeoutExpired(args, 1200)
                if failure == 'post-build-lock':
                    (source / 'Cargo.lock').write_bytes(b'changed lock')
                executable = Path(kwargs['env']['CARGO_TARGET_DIR']) / 'debug/examples' / ('baseline_compat_reader' + suffix)
                executable.parent.mkdir(parents=True)
                executable.write_bytes(b'synthetic reader identity')
                return b''
            raise AssertionError(f'unexpected command: {args}')

        def reader(args, **kwargs):
            state['readers'].append(kwargs['input'])
            self.assertEqual(kwargs['timeout'], 30)
            if failure == 'reader-timeout':
                raise subprocess.TimeoutExpired(args, 30)
            if failure == 'report-race':
                (root / 'report.json').write_bytes(b'concurrent evidence')
            if kwargs['input'] == FIXTURE.read_bytes():
                return subprocess.CompletedProcess(args, 0, b'BASELINE_V1_ACCEPTED\n', b'')
            return subprocess.CompletedProcess(args, 1, b'', b'BASELINE_V1_REJECTED: missing field `sanitized_ingress` at line 1 column 1\n')

        with patch.object(harness, 'LOCK', hashlib.sha256(lock_bytes).hexdigest()), \
                patch.object(harness, 'checked', side_effect=command), \
                patch.object(harness.subprocess, 'run', side_effect=reader):
            yield state

    def assert_clean_failure(self, state, output):
        self.assertFalse(output.exists(), 'failed run must not emit a success report')
        for directory in state['builds']:
            self.assertFalse(directory.exists(), 'task-created temporary build must be cleaned')

    def test_wrong_baseline_tree_fails_before_build_without_report(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root, tree='0' * 40) as state:
                with self.assertRaisesRegex(RuntimeError, 'baseline tree mismatch'):
                    harness.run(REPO, FIXTURE, output)
            self.assertEqual(state['builds'], [])
            self.assert_clean_failure(state, output)

    def test_wrong_baseline_lock_fails_before_build_without_report(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root, lock=b'wrong baseline lock') as state:
                with self.assertRaisesRegex(RuntimeError, 'baseline lockfile SHA-256 mismatch'):
                    harness.run(REPO, FIXTURE, output)
            self.assertEqual(state['builds'], [])
            self.assert_clean_failure(state, output)

    def test_wrong_rustc_or_cargo_version_fails_before_build_without_report(self):
        for versions in [{'rustc': 'rustc 1.96.0 synthetic\n'},
                         {'cargo': 'cargo 1.96.0 synthetic\n'}]:
            with self.subTest(versions=versions), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                output = root / 'report.json'
                with self.simulated_tools(root, **versions) as state:
                    with self.assertRaisesRegex(RuntimeError, 'requires Rust/Cargo 1.97.1'):
                        harness.run(REPO, FIXTURE, output)
                self.assertEqual(state['builds'], [])
                self.assert_clean_failure(state, output)

    def test_second_build_failure_does_not_certify_first_build(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root, failure='second-build') as state:
                with self.assertRaisesRegex(RuntimeError, 'second build failed'):
                    harness.run(REPO, FIXTURE, output)
            self.assertEqual(len(state['builds']), 2)
            self.assertNotEqual(*state['builds'])
            self.assertEqual(len(state['readers']), 2, 'only first build reached both reader checks')
            self.assert_clean_failure(state, output)

    def test_changed_post_build_lock_fails_without_report(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root, failure='post-build-lock') as state:
                with self.assertRaisesRegex(RuntimeError, 'post-build lockfile SHA-256 mismatch'):
                    harness.run(REPO, FIXTURE, output)
            self.assertEqual(len(state['builds']), 1)
            self.assertEqual(state['readers'], [])
            self.assert_clean_failure(state, output)

    def test_build_and_reader_timeouts_fail_without_report(self):
        for failure, timeout in [('build-timeout', 1200), ('reader-timeout', 30)]:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                output = root / 'report.json'
                with self.simulated_tools(root, failure=failure) as state:
                    with self.assertRaises(subprocess.TimeoutExpired) as caught:
                        harness.run(REPO, FIXTURE, output)
                self.assertEqual(caught.exception.timeout, timeout)
                self.assertEqual(len(state['builds']), 1)
                self.assert_clean_failure(state, output)

    def test_checked_preserves_command_timeout_and_deadline(self):
        error = subprocess.TimeoutExpired(['synthetic-command'], 1200)
        with patch.object(harness.subprocess, 'run', side_effect=error) as process:
            with self.assertRaises(subprocess.TimeoutExpired):
                harness.checked(['synthetic-command'])
        self.assertEqual(process.call_args.kwargs['timeout'], 1200)

    def test_timeout_is_cli_failure_without_success_output_or_report(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'report.json'
            stdout, stderr = io.StringIO(), io.StringIO()
            with patch.object(os.sys, 'argv', [str(SCRIPT), '--output', str(output)]), \
                    patch.object(harness, 'run', side_effect=subprocess.TimeoutExpired(['cargo'], 1200)), \
                    redirect_stdout(stdout), redirect_stderr(stderr):
                with self.assertRaises(SystemExit) as caught:
                    harness.main()
            self.assertEqual(caught.exception.code, 1)
            self.assertEqual(stdout.getvalue(), '')
            self.assertIn('baseline characterization FAILED:', stderr.getvalue())
            self.assertFalse(output.exists())

    def test_report_created_during_build_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root, failure='report-race') as state:
                with self.assertRaises(FileExistsError):
                    harness.run(REPO, FIXTURE, output)
            self.assertEqual(output.read_bytes(), b'concurrent evidence')
            self.assertEqual(len(state['builds']), 2)
            for directory in state['builds']:
                self.assertFalse(directory.exists())

    def test_successful_simulation_reaches_both_builds_and_report(self):
        # Positive control prevents mocks/setup bugs from masquerading as failures.
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / 'report.json'
            with self.simulated_tools(root) as state:
                report = harness.run(REPO, FIXTURE, output)
            self.assertEqual(len(state['builds']), 2)
            self.assertNotEqual(*state['builds'])
            self.assertEqual(len(state['readers']), 4)
            self.assertEqual(report['semantic_repeatability'], 'passed')
            self.assertFalse(report['historical_release_certification'])
            self.assertEqual(json.loads(output.read_text())['runs'], report['runs'])
            for directory in state['builds']:
                self.assertFalse(directory.exists())

    def test_framing_matches_reviewed_independent_golden_vector(self):
        value = json.loads(harness.envelope(b'{}'))
        self.assertEqual(value['seal_sha256'],
                         '0ddaed98f9efb4b8a89c37c2629b9456e7165dbb6cba1c4ca6d6f68120ec7ea0')
        self.assertEqual(set(value), {'format', 'normalization_profile', 'payload_json', 'seal_sha256'})

    def test_envelope_preserves_exact_payload_and_uses_utf8_lengths(self):
        raw = ' {"witness":"\U0001b123"} \n'.encode()
        value = json.loads(harness.envelope(raw))
        self.assertEqual(value['payload_json'].encode(), raw)
        parts = [harness.DOMAIN.encode(), harness.FORMAT.encode(), harness.PROFILE.encode(), raw]
        expected = hashlib.sha256(b''.join(str(len(p)).encode() + b':' + p for p in parts)).hexdigest()
        self.assertEqual(value['seal_sha256'], expected)

    def test_altered_fixture_fails_before_build(self):
        with tempfile.TemporaryDirectory() as temp:
            altered = Path(temp) / 'altered.json'
            altered.write_bytes(FIXTURE.read_bytes() + b'\n')
            output = Path(temp) / 'report.json'
            with self.assertRaisesRegex(RuntimeError, 'fixture SHA-256 mismatch'):
                harness.run(REPO, altered, output)
            self.assertFalse(output.exists())

    def test_missing_fixture_fails_without_report(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'report.json'
            with self.assertRaises(FileNotFoundError):
                harness.run(REPO, Path(temp) / 'missing', output)
            self.assertFalse(output.exists())

    def test_missing_executable_fails_without_report(self):
        with tempfile.TemporaryDirectory() as temp, patch.dict(os.environ, {'PATH': temp}):
            output = Path(temp) / 'report.json'
            with self.assertRaises(FileNotFoundError):
                harness.run(REPO, FIXTURE, output)
            self.assertFalse(output.exists())

    def test_build_failure_cannot_be_a_pass(self):
        with self.assertRaisesRegex(RuntimeError, 'failed'):
            harness.checked([os.sys.executable, '-c', 'raise SystemExit(17)'])

    def test_unexpected_reader_acceptance_crash_and_errors_fail(self):
        cases = [(0, b'BASELINE_V1_ACCEPTED\n', b'', False),
                 (1, b'', b'BASELINE_V1_REJECTED: integrity failure', False),
                 (-11, b'', b'', False), (0, b'', b'', True),
                 (0, b'BASELINE_V1_ACCEPTED\n', b'warning', True)]
        for code, out, err, accepted in cases:
            with self.subTest(code=code, accepted=accepted), self.assertRaisesRegex(RuntimeError, 'unexpected reader'):
                harness.adjudicate(subprocess.CompletedProcess([], code, out, err), accepted)

    def test_expected_results_are_classified(self):
        self.assertEqual(harness.adjudicate(subprocess.CompletedProcess([], 0, b'BASELINE_V1_ACCEPTED\n', b''), True), 'accepted')
        rejection = b'BASELINE_V1_REJECTED: missing field `sanitized_ingress` at line 1 column 1\n'
        self.assertEqual(harness.adjudicate(subprocess.CompletedProcess([], 1, b'', rejection), False), 'rejected_missing_sanitized_ingress')

    def test_existing_report_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / 'report.json'
            output.write_bytes(b'previous evidence')
            with self.assertRaisesRegex(RuntimeError, 'output already exists'):
                harness.run(REPO, FIXTURE, output)
            self.assertEqual(output.read_bytes(), b'previous evidence')

    def test_single_build_is_insufficient(self):
        with self.assertRaisesRegex(RuntimeError, 'two clean builds'):
            harness.run(REPO, FIXTURE, Path('unused'), repeats=1)


if __name__ == '__main__':
    unittest.main()
