use blake3::Hasher;

pub(crate) struct CanonicalHasher {
    hasher: Hasher,
}

impl CanonicalHasher {
    pub(crate) fn new(domain: &[u8]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&(domain.len() as u32).to_be_bytes());
        hasher.update(domain);
        Self { hasher }
    }

    pub(crate) fn field(&mut self, value: &[u8]) {
        self.hasher.update(&(value.len() as u32).to_be_bytes());
        self.hasher.update(value);
    }

    pub(crate) fn finish(self) -> String {
        self.hasher.finalize().to_hex().to_string()
    }
}
