//! Small shared SDK types (hashes and public keys).

/// A 32-byte hash (e.g. a transaction or block hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash256(pub [u8; 32]);

/// A 33-byte compressed ECDSA public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 33]);

/// A 20-byte hash (e.g. a contract or account script hash / `UInt160`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash160(pub [u8; 20]);
