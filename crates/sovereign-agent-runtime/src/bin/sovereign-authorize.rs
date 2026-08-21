use ed25519_dalek::{Signer, SigningKey, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sovereign_agent_runtime::capability::{
    derive_request_id, CapabilityRequest, CapabilityScope, CapabilityType, EvaluationResult,
    PolicyAuthority, RequestId, RequestStatus,
};
use sovereign_agent_runtime::execution::CapabilityRegistry;
use sovereign_agent_runtime::identity::{
    derive_identity_id, AgentClass, AgentIdentity, AgentIdentityStatus, Digest as IdentityDigest,
    PolicyId, ReplayTimestamp, SchemaVersion,
};
use sovereign_audit::RecordId;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

#[derive(Deserialize)]
struct Proposal {
    operation: String,
    target: String,
    report_sha256: String,
    requester_identity: String,
    authorization_nonce: String,
}

#[derive(Serialize)]
struct Receipt {
    contract_version: &'static str,
    authorized: bool,
    grant_id: String,
    proposal_sha256: String,
    operation: String,
    target: String,
    policy_id: String,
    requester_identity: String,
    capability_request_id: String,
    authorization_nonce: String,
    issued_at: String,
    expires_at: String,
    verifying_key: String,
    signature: String,
}

fn receipt_payload(receipt: &Receipt) -> Vec<u8> {
    let mut value = b"SOVEREIGN_AUTHORIZATION_RECEIPT_V3".to_vec();
    for field in [
        receipt.grant_id.as_str(),
        receipt.proposal_sha256.as_str(),
        receipt.operation.as_str(),
        receipt.target.as_str(),
        receipt.policy_id.as_str(),
        receipt.requester_identity.as_str(),
        receipt.capability_request_id.as_str(),
        receipt.authorization_nonce.as_str(),
        receipt.issued_at.as_str(),
        receipt.expires_at.as_str(),
    ] {
        value.extend_from_slice(&(field.len() as u64).to_be_bytes());
        value.extend_from_slice(field.as_bytes());
    }
    value
}

fn policy_seed() -> Result<[u8; 32], String> {
    let seed_hex = match (
        env::var("SOVEREIGN_POLICY_SEED_FILE"),
        env::var("SOVEREIGN_POLICY_SEED_HEX"),
    ) {
        (Ok(_), Ok(_)) => return Err("configure only one policy seed source".into()),
        (Ok(path), Err(_)) => {
            let path = PathBuf::from(path);
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| "policy seed file unreadable")?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("policy seed must be a regular non-symlink file".into());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err("policy seed file permissions are too broad".into());
                }
            }
            fs::read_to_string(path).map_err(|_| "policy seed file unreadable")?
        }
        (Err(_), Ok(value)) => value,
        (Err(_), Err(_)) => return Err("policy seed missing".into()),
    };
    hex::decode(seed_hex.trim())
        .map_err(|_| "invalid policy seed")?
        .try_into()
        .map_err(|_| "invalid policy seed length".into())
}

fn safe_target(target: &str, allowed_root: &Path) -> bool {
    let path = PathBuf::from(target);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return false;
    }
    let Ok(root) = fs::canonicalize(allowed_root) else {
        return false;
    };
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(parent) = fs::canonicalize(parent) else {
        return false;
    };
    parent.starts_with(root)
}

fn record_id(hex_digest: &str) -> Result<RecordId, String> {
    let bytes: [u8; 32] = hex::decode(hex_digest)
        .map_err(|_| "invalid record digest")?
        .try_into()
        .map_err(|_| "invalid record digest length")?;
    Ok(RecordId::from_bytes(bytes))
}

fn configured_requester_identity(
) -> Result<sovereign_agent_runtime::identity::AgentIdentityId, String> {
    let policy_id =
        env::var("SOVEREIGN_POLICY_ID").unwrap_or_else(|_| "local-workbench-policy-v1".into());
    let requester_key_digest =
        env::var("SOVEREIGN_REQUESTER_KEY_DIGEST").map_err(|_| "requester key digest missing")?;
    Ok(derive_identity_id(
        &IdentityDigest(requester_key_digest),
        &AgentClass::HumanOperator,
        &PolicyId(policy_id),
    ))
}

fn authorize(raw: &[u8]) -> Result<Receipt, String> {
    let proposal: Proposal = serde_json::from_slice(raw).map_err(|_| "malformed proposal")?;
    if proposal.report_sha256.len() != 64
        || !proposal
            .report_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid report identity".into());
    }
    if proposal.authorization_nonce.len() != 64
        || !proposal
            .authorization_nonce
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid authorization nonce".into());
    }
    let allowed_operations = env::var("SOVEREIGN_ALLOWED_OPERATIONS").unwrap_or_default();
    let allowed: Vec<_> = allowed_operations
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();
    if !allowed.contains(&proposal.operation.as_str()) {
        return Err("operation denied by policy".into());
    }
    let allowed_root = env::var("SOVEREIGN_ALLOWED_ROOT").map_err(|_| "allowed root missing")?;
    if !safe_target(&proposal.target, Path::new(&allowed_root)) {
        return Err("target outside allowed root".into());
    }
    let seed = policy_seed()?;
    let policy_id =
        env::var("SOVEREIGN_POLICY_ID").unwrap_or_else(|_| "local-workbench-policy-v1".into());
    let ttl_seconds: i64 = env::var("SOVEREIGN_GRANT_TTL_SECONDS")
        .unwrap_or_else(|_| "300".into())
        .parse()
        .map_err(|_| "invalid grant ttl")?;
    if !(1..=900).contains(&ttl_seconds) {
        return Err("grant ttl outside 1..900 seconds".into());
    }
    let requester_key_digest =
        env::var("SOVEREIGN_REQUESTER_KEY_DIGEST").map_err(|_| "requester key digest missing")?;
    let digest = IdentityDigest(requester_key_digest);
    let governing_policy = PolicyId(policy_id.clone());
    let requester_id = derive_identity_id(&digest, &AgentClass::HumanOperator, &governing_policy);
    if requester_id.0 != proposal.requester_identity {
        return Err("requester identity mismatch".into());
    }
    let proposal_digest = Sha256::digest(raw);
    let proposal_sha256 = hex::encode(proposal_digest);
    let issued = chrono::Utc::now();
    let issued_at = issued.to_rfc3339();
    let expires_at = (issued + chrono::Duration::seconds(ttl_seconds)).to_rfc3339();
    let signing_key = SigningKey::from_bytes(&seed);
    let authority = PolicyAuthority::from_seed(policy_id.clone(), seed);
    let capability_type = CapabilityType::Write;
    let scope = CapabilityScope {
        target_resource: proposal.target.clone(),
        target_reference: record_id(&proposal.report_sha256)?,
        permissions: vec!["write".into()],
    };
    let mut request = CapabilityRequest {
        schema_version: "CAPABILITY_REQUEST-v1".into(),
        request_id: RequestId(String::new()),
        requester_identity: requester_id.clone(),
        proposal_reference: record_id(&proposal_sha256)?,
        capability_type,
        requested_scope: scope.clone(),
        created_at: issued_at.clone(),
        status: RequestStatus::Pending,
    };
    request.request_id = derive_request_id(
        &request.requester_identity,
        &request.proposal_reference,
        &request.capability_type,
        &request.requested_scope,
        &request.created_at,
    );
    let evaluation = authority.evaluate(
        &request,
        EvaluationResult::Approved {
            scope,
            expires_at: expires_at.clone(),
        },
        issued_at.clone(),
    );
    let identity = AgentIdentity {
        schema_version: SchemaVersion("AGENT_IDENTITY-v1".into()),
        identity_id: requester_id.clone(),
        public_key_digest: digest,
        agent_class: AgentClass::HumanOperator,
        created_at: ReplayTimestamp(issued_at.clone()),
        governing_policy,
        status: AgentIdentityStatus::Active,
    };
    let mut registry = CapabilityRegistry::new(policy_id.clone(), authority.verifying_key());
    registry
        .register_identity(identity)
        .map_err(|_| "requester identity registration failed")?;
    let grant_id = registry
        .admit(&request, &evaluation, &issued_at)
        .map_err(|_| "capability registry denied grant")?
        .0;
    let mut receipt = Receipt {
        contract_version: "sovereign.authorization.receipt.v3",
        authorized: true,
        grant_id,
        proposal_sha256,
        operation: proposal.operation,
        target: proposal.target,
        policy_id,
        requester_identity: requester_id.0,
        capability_request_id: request.request_id.0,
        authorization_nonce: proposal.authorization_nonce,
        issued_at,
        expires_at,
        verifying_key: hex::encode(signing_key.verifying_key().to_bytes()),
        signature: String::new(),
    };
    let payload = receipt_payload(&receipt);
    let signature = signing_key.sign(&payload);
    signing_key
        .verifying_key()
        .verify(&payload, &signature)
        .map_err(|_| "self-verification failed")?;
    receipt.signature = hex::encode(signature.to_bytes());
    Ok(receipt)
}

fn main() {
    if env::args().nth(1).as_deref() == Some("--print-requester-identity") {
        match configured_requester_identity() {
            Ok(identity) => {
                println!("{}", identity.0);
                return;
            }
            Err(error) => {
                eprintln!("identity derivation failed: {error}");
                std::process::exit(2);
            }
        }
    }
    let mut raw = Vec::new();
    if io::stdin().read_to_end(&mut raw).is_err() {
        std::process::exit(3);
    }
    match authorize(&raw) {
        Ok(receipt) => println!(
            "{}",
            serde_json::to_string(&receipt).expect("receipt serialization")
        ),
        Err(error) => {
            eprintln!("authorization denied: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_traversal_is_denied() {
        let root = env::temp_dir().join("sovereign-authorize-traversal");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("../escape.json");
        assert!(!safe_target(target.to_str().unwrap(), &root));
    }

    #[test]
    fn exact_descendant_is_allowed() {
        let root = env::temp_dir().join("sovereign-authorize-descendant");
        fs::create_dir_all(&root).unwrap();
        let target = root.join("review.json");
        assert!(safe_target(target.to_str().unwrap(), &root));
    }

    #[test]
    fn requester_identity_is_stable() {
        let digest = IdentityDigest("11".repeat(32));
        let first = derive_identity_id(
            &digest,
            &AgentClass::HumanOperator,
            &PolicyId("policy".into()),
        );
        let second = derive_identity_id(
            &digest,
            &AgentClass::HumanOperator,
            &PolicyId("policy".into()),
        );
        assert_eq!(first, second);
        assert_eq!(first.0.len(), 64);
    }
}
