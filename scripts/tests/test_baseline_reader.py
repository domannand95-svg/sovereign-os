"""Failure and framing contracts; no toolchain, network or provider needed."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'verify-baseline-reader.py'
spec = importlib.util.spec_from_file_location('baseline_reader', SCRIPT)
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)
REPO = SCRIPT.parent.parent
FIXTURE = REPO / 'crates/beta001-harness/tests/fixtures/unicode17_v1_package.json'


class BaselineReaderTests(unittest.TestCase):
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
