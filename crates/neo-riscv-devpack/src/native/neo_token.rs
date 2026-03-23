use alloc::vec::Vec;

use neo_riscv_abi::StackValue;

use super::{
    call_native, call_native_read_only, stack_item_as_bool, stack_item_as_fixed_bytes,
    stack_item_as_i64, stack_item_as_string, stack_item_as_u8,
};

// NeoToken native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const NEO_TOKEN_HASH: [u8; 20] = [
    0xf5, 0x63, 0xea, 0x40, 0xbc, 0x28, 0x3d, 0x4d, 0x0e, 0x05, 0xc4, 0x8e, 0xa3, 0x05, 0xb3, 0xf2,
    0xa0, 0x73, 0x40, 0xef,
];

pub fn neo_balance_of(account: &[u8; 20]) -> i64 {
    let args = [StackValue::ByteString(account.to_vec())];
    call_native_read_only(&NEO_TOKEN_HASH, "balanceOf", &args)
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn neo_transfer(from: &[u8; 20], to: &[u8; 20], amount: i64) -> bool {
    let args = [
        StackValue::ByteString(from.to_vec()),
        StackValue::ByteString(to.to_vec()),
        StackValue::Integer(amount),
        StackValue::Null,
    ];
    call_native(&NEO_TOKEN_HASH, "transfer", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn neo_get_candidates() -> Vec<([u8; 33], i64)> {
    let value = match call_native_read_only(&NEO_TOKEN_HASH, "getCandidates", &[]) {
        Some(value) => value,
        None => return Vec::new(),
    };

    let items = match value {
        StackValue::Array(items) | StackValue::Struct(items) => items,
        _ => return Vec::new(),
    };

    items
        .into_iter()
        .filter_map(|candidate| match candidate {
            StackValue::Struct(fields) | StackValue::Array(fields) if fields.len() >= 2 => {
                let pubkey = stack_item_as_fixed_bytes::<33>(&fields[0])?;
                let votes = stack_item_as_i64(&fields[1])?;
                Some((pubkey, votes))
            }
            _ => None,
        })
        .collect()
}

pub fn neo_register_candidate(pubkey: &[u8; 33]) -> bool {
    let args = [StackValue::ByteString(pubkey.to_vec())];
    call_native(&NEO_TOKEN_HASH, "registerCandidate", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn neo_vote(account: &[u8; 20], pubkey: &[u8; 33]) -> bool {
    let args = [
        StackValue::ByteString(account.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
    ];
    call_native(&NEO_TOKEN_HASH, "vote", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn neo_unclaimed_gas(account: &[u8; 20], end: u32) -> i64 {
    let args = [
        StackValue::ByteString(account.to_vec()),
        StackValue::Integer(i64::from(end)),
    ];
    call_native_read_only(&NEO_TOKEN_HASH, "unclaimedGas", &args)
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn neo_symbol() -> &'static str {
    const DEFAULT: &str = "NEO";
    let actual = call_native_read_only(&NEO_TOKEN_HASH, "symbol", &[])
        .and_then(|v| stack_item_as_string(&v));
    match actual.as_deref() {
        Some("NEO") => DEFAULT,
        _ => DEFAULT,
    }
}

pub fn neo_decimals() -> u8 {
    call_native_read_only(&NEO_TOKEN_HASH, "decimals", &[])
        .and_then(|v| stack_item_as_u8(&v))
        .unwrap_or(0)
}

pub fn neo_total_supply() -> i64 {
    call_native_read_only(&NEO_TOKEN_HASH, "totalSupply", &[])
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(100_000_000)
}
