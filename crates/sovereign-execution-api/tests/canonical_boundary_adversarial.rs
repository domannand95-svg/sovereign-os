use sovereign_execution_api::{BoundaryParseError, CanonicalAction, DigestRef};

fn valid_bytes() -> Vec<u8> {
    use sha2::Digest;
    let payload = b"bounded payload";
    let digest = DigestRef::from_bytes(sha2::Sha256::digest(payload).into());
    CanonicalAction::encode("write", "/bounded/target", digest, payload)
        .unwrap()
        .as_bytes()
        .to_vec()
}

#[test]
fn canonical_round_trip_is_byte_exact() {
    let bytes = valid_bytes();
    let parsed = CanonicalAction::parse(&bytes).unwrap();
    assert_eq!(parsed.as_bytes(), bytes);
}

#[test]
fn malformed_inputs_never_panic() {
    let bytes = valid_bytes();
    for end in 0..bytes.len() {
        let result = std::panic::catch_unwind(|| CanonicalAction::parse(&bytes[..end]));
        assert!(matches!(result, Ok(Err(_))));
    }
}

#[test]
fn deserialization_cannot_bypass_canonical_parser() {
    assert!(serde_json::from_str::<CanonicalAction>("[1,2,3]").is_err());
}

#[test]
fn trailing_bytes_are_rejected() {
    let mut bytes = valid_bytes();
    bytes.push(0);
    assert_eq!(
        CanonicalAction::parse(&bytes),
        Err(BoundaryParseError::TrailingBytes)
    );
}

#[test]
fn unknown_version_is_rejected() {
    let mut bytes = valid_bytes();
    bytes[6] = 255;
    assert_eq!(
        CanonicalAction::parse(&bytes),
        Err(BoundaryParseError::UnsupportedVersion(255))
    );
}

#[test]
fn forged_payload_digest_is_rejected() {
    let mut bytes = valid_bytes();
    let payload_index = bytes.len() - b"bounded payload".len();
    bytes[payload_index] ^= 1;
    assert_eq!(
        CanonicalAction::parse(&bytes),
        Err(BoundaryParseError::DigestMismatch)
    );
}

#[test]
fn declared_oversize_payload_is_rejected_without_allocation() {
    let mut bytes = valid_bytes();
    let length_index = bytes.len() - b"bounded payload".len() - 4;
    bytes[length_index..length_index + 4].copy_from_slice(&1_048_577_u32.to_be_bytes());
    assert_eq!(
        CanonicalAction::parse(&bytes),
        Err(BoundaryParseError::PayloadTooLarge)
    );
}
