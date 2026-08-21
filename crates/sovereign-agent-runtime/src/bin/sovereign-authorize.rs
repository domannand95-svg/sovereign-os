use ed25519_dalek::{Signer, SigningKey, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

#[derive(Deserialize)]
struct Proposal {
    operation: String,
    target: String,
    report_sha256: String,
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
    verifying_key: String,
    signature: String,
}

fn receipt_payload(
    grant_id: &str,
    proposal_sha256: &str,
    operation: &str,
    target: &str,
    policy_id: &str,
) -> Vec<u8> {
    let mut value = b"SOVEREIGN_AUTHORIZATION_RECEIPT_V2".to_vec();
    for field in [grant_id, proposal_sha256, operation, target, policy_id] {
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
    path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        && path.starts_with(allowed_root)
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
    let proposal_digest = Sha256::digest(raw);
    let proposal_sha256 = hex::encode(proposal_digest);
    let mut grant_hasher = blake3::Hasher::new();
    grant_hasher.update(b"SOVEREIGN_LOCAL_AUTHORIZER_GRANT_V1");
    grant_hasher.update(&proposal_digest);
    grant_hasher.update(policy_id.as_bytes());
    let grant_id = grant_hasher.finalize().to_hex().to_string();
    let signing_key = SigningKey::from_bytes(&seed);
    let payload = receipt_payload(
        &grant_id,
        &proposal_sha256,
        &proposal.operation,
        &proposal.target,
        &policy_id,
    );
    let signature = signing_key.sign(&payload);
    signing_key
        .verifying_key()
        .verify(&payload, &signature)
        .map_err(|_| "self-verification failed")?;
    Ok(Receipt {
        contract_version: "sovereign.authorization.receipt.v2",
        authorized: true,
        grant_id,
        proposal_sha256,
        operation: proposal.operation,
        target: proposal.target,
        policy_id,
        verifying_key: hex::encode(signing_key.verifying_key().to_bytes()),
        signature: hex::encode(signature.to_bytes()),
    })
}

fn main() {
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
        let root = env::current_dir().unwrap().join("approved");
        let target = root.join("../escape.json");
        assert!(!safe_target(target.to_str().unwrap(), &root));
    }

    #[test]
    fn exact_descendant_is_allowed() {
        let root = env::current_dir().unwrap().join("approved");
        let target = root.join("review.json");
        assert!(safe_target(target.to_str().unwrap(), &root));
    }
}
