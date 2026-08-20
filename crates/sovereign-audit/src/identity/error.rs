#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    InvalidIdentityId,
    MalformedKeyDigest,
}
