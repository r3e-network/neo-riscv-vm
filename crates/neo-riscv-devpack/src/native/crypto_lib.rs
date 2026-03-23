use neo_riscv_abi::StackValue;

use super::{call_native_read_only, stack_item_as_bool, stack_item_as_fixed_bytes};

// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const CRYPTO_LIB_HASH: [u8; 20] = [
    0x1b, 0xf5, 0x75, 0xab, 0x11, 0x89, 0x68, 0x84, 0x13, 0x61, 0x0a, 0x35, 0xa1, 0x28, 0x86, 0xcd,
    0xe0, 0xb6, 0x6c, 0x72,
];

const DEFAULT_ECDSA_CURVE_HASH_SECP256R1_SHA256: i64 = 23;

// CryptoLib native contract bindings
pub fn crypto_sha256(data: &[u8]) -> [u8; 32] {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&CRYPTO_LIB_HASH, "sha256", &args)
        .and_then(|v| stack_item_as_fixed_bytes::<32>(&v))
        .unwrap_or([0; 32])
}

pub fn crypto_ripemd160(data: &[u8]) -> [u8; 20] {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&CRYPTO_LIB_HASH, "ripemd160", &args)
        .and_then(|v| stack_item_as_fixed_bytes::<20>(&v))
        .unwrap_or([0; 20])
}

pub fn crypto_verify_with_ecdsa(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool {
    // The native contract API requires a `curveHash` parameter. The current Rust wrapper keeps the
    // preexisting signature and defaults to secp256r1+SHA256 (the common Neo key curve).
    let args = [
        StackValue::ByteString(message.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
        StackValue::ByteString(signature.to_vec()),
        StackValue::Integer(DEFAULT_ECDSA_CURVE_HASH_SECP256R1_SHA256),
    ];
    call_native_read_only(&CRYPTO_LIB_HASH, "verifyWithECDsa", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn crypto_murmur32(data: &[u8], seed: u32) -> [u8; 4] {
    let args = [
        StackValue::ByteString(data.to_vec()),
        StackValue::Integer(i64::from(seed)),
    ];
    call_native_read_only(&CRYPTO_LIB_HASH, "murmur32", &args)
        .and_then(|v| stack_item_as_fixed_bytes::<4>(&v))
        .unwrap_or([0; 4])
}

pub fn crypto_keccak256(data: &[u8]) -> [u8; 32] {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&CRYPTO_LIB_HASH, "keccak256", &args)
        .and_then(|v| stack_item_as_fixed_bytes::<32>(&v))
        .unwrap_or([0; 32])
}

pub fn crypto_verify_with_ed25519(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool {
    let args = [
        StackValue::ByteString(message.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
        StackValue::ByteString(signature.to_vec()),
    ];
    call_native_read_only(&CRYPTO_LIB_HASH, "verifyWithEd25519", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}
