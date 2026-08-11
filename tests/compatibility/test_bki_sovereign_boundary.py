import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker


SOVEREIGN_ROOT = Path(__file__).resolve().parents[2]
BKI_ROOT = Path(os.environ["BKI_ROOT"]).resolve()
PINNED_BKI_COMMIT = "0ace31f9071969825b75187d31c5f418212e9ae9"
PROFILE_SCHEMA_SHA256 = (
    "1a513455e09063f41d03023688ffc7b89bdababaf8ed9d1a78c101edb7b8845d"
)

sys.path.insert(0, str(BKI_ROOT))

from tooling.integration import ProfileTranslationError, translate_frontmatter  # noqa: E402


def _schema(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _canonical_lf(content: bytes) -> bytes:
    return content.replace(b"\r\n", b"\n").replace(b"\r", b"\n")


def _checkout_head(repository: Path) -> str:
    head = (repository / ".git" / "HEAD").read_text(encoding="ascii").strip()
    if not head.startswith("ref: "):
        return head

    ref_name = head.removeprefix("ref: ")
    loose_ref = repository / ".git" / Path(ref_name)
    if loose_ref.exists():
        return loose_ref.read_text(encoding="ascii").strip()

    packed_refs = (repository / ".git" / "packed-refs").read_text(
        encoding="ascii"
    )
    for line in packed_refs.splitlines():
        if line.endswith(f" {ref_name}"):
            return line.split(" ", 1)[0]
    raise AssertionError(f"Unable to resolve checkout ref: {ref_name}")


def _run_cli(source: Path, candidate: Path, contract: str):
    return subprocess.run(
        [
            sys.executable,
            "-m",
            "tooling.normalization.cli",
            "--source",
            str(source),
            "--candidate",
            str(candidate),
            "--format",
            contract,
        ],
        cwd=BKI_ROOT,
        capture_output=True,
        check=False,
        timeout=10,
    )


def _canonical_document() -> bytes:
    return (
        "---\n"
        "document_id: SOS-COMPAT-001\n"
        "version: 1.0\n"
        "status: Compatibility Fixture\n"
        "last_revised: 2026-08-11\n"
        "---\n\n"
        "# Compatibility Fixture\n\n"
        "Stable governed content.\n"
    ).encode("utf-8")


def test_pinned_bki_checkout_is_exact_commit():
    assert _checkout_head(BKI_ROOT) == PINNED_BKI_COMMIT


def test_profile_schema_bytes_and_digest_match():
    bki_schema = (
        BKI_ROOT / "docs" / "integration" / "bki-sovereign-profile-v1.schema.json"
    ).read_bytes()
    sovereign_schema = (
        SOVEREIGN_ROOT
        / "docs"
        / "integration"
        / "bki-sovereign-profile-v1.schema.json"
    ).read_bytes()

    canonical_bki_schema = _canonical_lf(bki_schema)
    canonical_sovereign_schema = _canonical_lf(sovereign_schema)
    assert canonical_sovereign_schema == canonical_bki_schema
    assert hashlib.sha256(canonical_bki_schema).hexdigest() == PROFILE_SCHEMA_SHA256


def test_actual_a04_spec_translates_to_namespaced_profile():
    spec = (
        SOVEREIGN_ROOT
        / "docs"
        / "specs"
        / "SPEC-EV-001-Governed-Research-Artifacts-and-Evidence.md"
    ).read_text(encoding="utf-8")
    translated = translate_frontmatter(
        spec,
        source_format="sovereign.document.v1",
    )

    Draft202012Validator(
        _schema(
            BKI_ROOT
            / "docs"
            / "integration"
            / "bki-sovereign-profile-v1.schema.json"
        ),
        format_checker=FormatChecker(),
    ).validate(translated)
    assert translated == {
        "profile_version": "bki.sovereign.profile.v1",
        "document_id": "SPEC-EV-001",
        "version": "0.1",
        "status": {
            "namespace": "sovereign",
            "value": "Approved Implementation Baseline",
        },
        "last_revised": "2026-08-11",
    }


def test_compliant_cli_result_is_schema_valid_and_hash_exact(tmp_path):
    content = _canonical_document()
    source = tmp_path / "source.md"
    candidate = tmp_path / "candidate.md"
    source.write_bytes(content)
    candidate.write_bytes(content)
    before = {path.name: path.read_bytes() for path in tmp_path.iterdir()}

    completed = _run_cli(source, candidate, "bki.validation.v1")

    assert completed.returncode == 0
    assert completed.stderr == b""
    payload = json.loads(completed.stdout.decode("utf-8"))
    Draft202012Validator(
        _schema(
            BKI_ROOT
            / "docs"
            / "integration"
            / "bki-validation-result-v1.schema.json"
        ),
        format_checker=FormatChecker(),
    ).validate(payload)
    expected_hash = hashlib.sha256(content).hexdigest()
    assert payload["outcome"] == "PASS — COMPLIANT"
    assert payload["source_sha256"] == expected_hash
    assert payload["candidate_sha256"] == expected_hash
    assert {path.name: path.read_bytes() for path in tmp_path.iterdir()} == before


def test_quarantine_and_unknown_contract_fail_closed(tmp_path):
    source = tmp_path / "source.md"
    candidate = tmp_path / "candidate.md"
    source.write_bytes(_canonical_document())
    candidate.write_bytes(
        _canonical_document().replace(b"Stable governed", b"Mutated governed")
    )

    quarantine = _run_cli(source, candidate, "bki.validation.v1")
    unknown = _run_cli(source, source, "bki.validation.v2")

    assert quarantine.returncode == 2
    assert json.loads(quarantine.stdout.decode("utf-8"))["outcome"] == (
        "FAIL — QUARANTINE"
    )
    assert unknown.returncode == 3
    assert unknown.stdout == b""


@pytest.mark.parametrize(
    "mutation",
    [
        "ID: SPEC-EV-001\ndocument_id: SPEC-EV-001",
        "ID: SPEC-EV-001\nID: OTHER",
    ],
)
def test_metadata_confusion_attempts_fail_closed(mutation):
    spec = (
        SOVEREIGN_ROOT
        / "docs"
        / "specs"
        / "SPEC-EV-001-Governed-Research-Artifacts-and-Evidence.md"
    ).read_text(encoding="utf-8")
    tampered = spec.replace("ID: SPEC-EV-001", mutation)

    with pytest.raises(ProfileTranslationError):
        translate_frontmatter(
            tampered,
            source_format="sovereign.document.v1",
        )


def test_malformed_or_partial_json_cannot_be_consumed():
    validator = Draft202012Validator(
        _schema(
            BKI_ROOT
            / "docs"
            / "integration"
            / "bki-validation-result-v1.schema.json"
        ),
        format_checker=FormatChecker(),
    )

    with pytest.raises(json.JSONDecodeError):
        json.loads('{"contract_version":"bki.validation.v1"')
    assert list(validator.iter_errors({"contract_version": "bki.validation.v1"}))
