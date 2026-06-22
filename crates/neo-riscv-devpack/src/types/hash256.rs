//! Shared SDK type: a 32-byte hash.

/// A 32-byte hash (e.g. a transaction or block hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash256(pub [u8; 32]);
