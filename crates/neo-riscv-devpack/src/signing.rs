use alloc::vec::Vec;

use crate::syscalls::{crypto_verify_signature, runtime_check_witness};

/// Verify a signature against a message using the host's crypto implementation.
/// Returns true if the signature is valid for the given public key.
pub fn verify_signature(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool {
    crypto_verify_signature(message, pubkey, signature)
}

/// Check whether the given hash (UInt160 or ECPoint) is a valid witness
/// for the current transaction.
pub fn check_witness(hash: &[u8]) -> bool {
    runtime_check_witness(hash)
}

/// Verify a multi-signature against a message.
pub fn verify_multisig(
    message: &[u8],
    pubkeys: &[Vec<u8>],
    signatures: &[Vec<u8>],
) -> bool {
    crate::syscalls::crypto_check_multisig(message, pubkeys, signatures)
}
