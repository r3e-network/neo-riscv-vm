//! Shared SDK type: a 33-byte compressed public key.

/// A 33-byte compressed ECDSA public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 33]);
