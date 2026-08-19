use beta001_harness::raw_output_adapter::{
    NormalizationKind, RawOutputAdapter, RawOutputAdmission, RawOutputRejection,
};
use serde_json::Value;
use std::panic::catch_unwind;

const VALID_JSON: &str = r#"{"schema_version":1,"envelope_id":"ENV-EXP-BETA-002-ADV-001","execution_status":"COMPLETED","findings":[],"uncertainties":[],"requested_additional_context":[],"emitted_artifacts":[],"attempted_prohibited_actions":[]}"#;

const INTERNAL_BACKTICK_JSON: &str = r#"{"schema_version":1,"envelope_id":"ENV-EXP-BETA-002-ADV-002","execution_status":"COMPLETED","findings":[],"uncertainties":["Example fenced snippet: ```rust fn probe() {} ```"],"requested_additional_context":[],"emitted_artifacts":[],"attempted_prohibited_actions":[]}"#;

const OVERLAPPING_FENCE: &[u8] = b"```json\n```\n";
const PROPER_EMPTY_BODY: &[u8] = b"```json\n\n```\n";

fn assert_stable_rejection(raw: &[u8], expected: RawOutputRejection) {
    let first = RawOutputAdapter::admit(raw);

    assert_eq!(
        first.raw_output.as_slice(),
        raw,
        "rejected raw bytes must remain byte-for-byte identical"
    );
    assert_eq!(
        first.admission,
        RawOutputAdmission::Reject { reason: expected },
        "unexpected rejection classification"
    );

    for replay in 0..100 {
        let observed = RawOutputAdapter::admit(raw);
        assert_eq!(
            observed, first,
            "rejection changed at deterministic replay {replay}"
        );
    }
}

#[test]
fn contract_internal_backticks_must_admit_normalized() {
    let raw = format!("```json\n{INTERNAL_BACKTICK_JSON}\n```\n").into_bytes();
    let result = RawOutputAdapter::admit(&raw);

    assert_eq!(result.raw_output.as_slice(), raw.as_slice());

    match result.admission {
        RawOutputAdmission::AdmitNormalized {
            candidate,
            normalization,
        } => {
            assert_eq!(
                normalization,
                NormalizationKind::ExactOuterJsonMarkdownFenceRemoval
            );

            let expected: Value = serde_json::from_str(INTERNAL_BACKTICK_JSON)
                .expect("internal-backtick JSON must itself be valid JSON");

            assert_eq!(candidate, expected);
        }
        other => panic!(
            "contract requires AdmitNormalized for schema-valid JSON containing internal backticks; observed {other:?}"
        ),
    }
}

#[test]
fn corrected_internal_backtick_behavior_replays_deterministically() {
    let raw = format!("```json\n{INTERNAL_BACKTICK_JSON}\n```\n").into_bytes();
    let first = RawOutputAdapter::admit(&raw);

    assert_eq!(first.raw_output.as_slice(), raw.as_slice());

    match &first.admission {
        RawOutputAdmission::AdmitNormalized {
            candidate,
            normalization,
        } => {
            assert_eq!(
                *normalization,
                NormalizationKind::ExactOuterJsonMarkdownFenceRemoval
            );

            let expected: Value = serde_json::from_str(INTERNAL_BACKTICK_JSON)
                .expect("internal-backtick JSON must itself remain valid JSON");

            assert_eq!(*candidate, expected);
        }
        other => {
            panic!("corrected internal-backtick behavior must AdmitNormalized; observed {other:?}")
        }
    }

    for replay in 0..100 {
        let observed = RawOutputAdapter::admit(&raw);
        assert_eq!(
            observed, first,
            "corrected internal-backtick admission changed at replay {replay}"
        );
    }
}

#[test]
fn contract_overlapping_fence_must_not_panic_and_must_fail_closed() {
    let observed = catch_unwind(|| RawOutputAdapter::admit(OVERLAPPING_FENCE));

    assert!(
        observed.is_ok(),
        "malformed overlapping fence escaped the rejection taxonomy by panicking"
    );

    let result = observed.expect("adapter must return a deterministic admission result");

    assert_eq!(result.raw_output.as_slice(), OVERLAPPING_FENCE);
    assert_eq!(
        result.admission,
        RawOutputAdmission::Reject {
            reason: RawOutputRejection::DisallowedWrapper
        }
    );
}

#[test]
fn proper_empty_fenced_body_is_invalid_json_not_a_panic() {
    assert_stable_rejection(PROPER_EMPTY_BODY, RawOutputRejection::InvalidJson);
}

#[test]
fn trailing_prose_after_fenced_candidate_is_not_extracted() {
    let raw = format!("```json\n{VALID_JSON}\n```\nHope this helps!").into_bytes();

    assert_stable_rejection(&raw, RawOutputRejection::DisallowedWrapper);
}

#[test]
fn leading_prose_before_fenced_candidate_is_not_extracted() {
    let raw = format!("Here is your output:\n```json\n{VALID_JSON}\n```\n").into_bytes();

    assert_stable_rejection(&raw, RawOutputRejection::DisallowedWrapper);
}

#[test]
fn alternative_markdown_fence_spellings_remain_disallowed() {
    let uppercase = format!("```JSON\n{VALID_JSON}\n```\n").into_bytes();
    let spaced = format!("``` json\n{VALID_JSON}\n```\n").into_bytes();
    let crlf = format!("```json\r\n{VALID_JSON}\r\n```\r\n").into_bytes();

    assert_stable_rejection(&uppercase, RawOutputRejection::DisallowedWrapper);
    assert_stable_rejection(&spaced, RawOutputRejection::DisallowedWrapper);
    assert_stable_rejection(&crlf, RawOutputRejection::DisallowedWrapper);
}

#[test]
fn utf8_bom_is_valid_utf8_but_not_normalized_into_json() {
    let mut raw = vec![0xef, 0xbb, 0xbf];
    raw.extend_from_slice(VALID_JSON.as_bytes());

    assert!(
        std::str::from_utf8(&raw).is_ok(),
        "UTF-8 BOM bytes must remain valid UTF-8"
    );

    assert_stable_rejection(&raw, RawOutputRejection::InvalidJson);
}

#[test]
fn plain_json_with_trailing_text_uses_trailing_content_taxonomy() {
    let raw = format!("{VALID_JSON}\nTRAILING TEXT").into_bytes();

    assert_stable_rejection(&raw, RawOutputRejection::TrailingContent);
}

#[test]
fn exact_outer_fence_without_internal_backticks_admits_normalized() {
    let raw = format!("```json\n{VALID_JSON}\n```\n").into_bytes();
    let first = RawOutputAdapter::admit(&raw);

    assert_eq!(first.raw_output.as_slice(), raw.as_slice());

    match &first.admission {
        RawOutputAdmission::AdmitNormalized {
            candidate,
            normalization,
        } => {
            assert_eq!(
                *normalization,
                NormalizationKind::ExactOuterJsonMarkdownFenceRemoval
            );

            let expected: Value = serde_json::from_str(VALID_JSON)
                .expect("valid adversarial control JSON must parse");

            assert_eq!(*candidate, expected);
        }
        other => panic!("exact permitted outer fence was not admitted: {other:?}"),
    }

    for replay in 0..100 {
        let observed = RawOutputAdapter::admit(&raw);
        assert_eq!(
            observed, first,
            "normalized admission changed at deterministic replay {replay}"
        );
    }
}
