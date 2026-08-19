use beta001_harness::raw_output_adapter::{
    RawOutputAdapter, RawOutputAdmission, RawOutputRejection,
};

fn assert_rejection_is_stable(raw: &[u8], expected: RawOutputRejection) {
    let first = RawOutputAdapter::admit(raw);

    assert_eq!(
        first.raw_output.as_slice(),
        raw,
        "original rejected bytes must be preserved byte-for-byte"
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
            "rejection classification changed at replay {replay}"
        );
    }
}

#[test]
fn invalid_utf8_is_rejected_deterministically() {
    const RAW: &[u8] = &[0xff, 0xfe, 0xfd];

    assert_rejection_is_stable(RAW, RawOutputRejection::InvalidUtf8);
}

#[test]
fn malformed_json_is_rejected_deterministically() {
    const RAW: &[u8] = br#"{"schema_version":1"#;

    assert_rejection_is_stable(RAW, RawOutputRejection::InvalidJson);
}

#[test]
fn non_permitted_markdown_wrapper_is_rejected_deterministically() {
    const RAW: &[u8] = b"```text\n{}\n```\n";

    assert_rejection_is_stable(RAW, RawOutputRejection::DisallowedWrapper);
}
