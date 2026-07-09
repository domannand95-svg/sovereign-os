use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caid(pub [u8; 32]);

impl Caid {
    pub fn from_payload(payload: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(payload);
        let hash_output = hasher.finalize();
        Self(*hash_output.as_bytes())
    }
}

impl fmt::Display for Caid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0.iter() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_caid_derivation() {
        let payload_a = b"sovereign_os_capability_node_specification_v1";
        let payload_b = b"sovereign_os_capability_node_specification_v2";

        let caid_a1 = Caid::from_payload(payload_a);
        let caid_a2 = Caid::from_payload(payload_a);
        let caid_b = Caid::from_payload(payload_b);

        assert_eq!(caid_a1, caid_a2);
        assert_ne!(caid_a1, caid_b);
    }

    #[test]
    fn canonical_hexadecimal_string_formatting() {
        let raw_bytes = [0xAB_u8; 32];
        let caid = Caid(raw_bytes);
        let formatted = caid.to_string();

        assert_eq!(formatted.len(), 64);
        assert!(formatted.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(&formatted[0..4], "abab");
    }
}
