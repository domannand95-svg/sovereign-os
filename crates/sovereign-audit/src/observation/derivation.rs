use super::model::{ObservationId, ObservationType, PayloadDigest};
use blake3::Hasher;

pub fn derive_observation_id(
    observer_id: &str,
    observation_type: &ObservationType,
    payload_digest: &PayloadDigest,
    timestamp: &str,
) -> ObservationId {
    let mut hasher = Hasher::new();

    hasher.update(b"EVIDENCE_OBSERVATION_v1");
    hasher.update(observer_id.as_bytes());
    hasher.update(observation_type.canonical_bytes());
    hasher.update(payload_digest.0.as_bytes());
    hasher.update(timestamp.as_bytes());

    ObservationId(hex::encode(hasher.finalize().as_bytes()))
}
