#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash160(pub [u8; 20]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash256(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 33]);
