mod hash256;
mod public_key;

pub use hash256::Hash256;
pub use public_key::PublicKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash160(pub [u8; 20]);
