//! Small shared SDK types (hashes and public keys).

mod hash256;
mod public_key;

/// Re-export of [`Hash256`](hash256::Hash256).
pub use hash256::Hash256;
/// Re-export of [`PublicKey`](public_key::PublicKey).
pub use public_key::PublicKey;

/// A 20-byte hash (e.g. a contract or account script hash / `UInt160`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash160(pub [u8; 20]);
