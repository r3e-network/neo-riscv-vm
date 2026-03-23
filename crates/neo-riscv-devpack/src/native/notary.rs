use neo_riscv_abi::StackValue;

use super::{
    call_native, call_native_read_only, stack_item_as_bool, stack_item_as_i64, stack_item_as_u32,
};

// Notary native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const NOTARY_HASH: [u8; 20] = [
    0x3b, 0xec, 0x35, 0x31, 0x11, 0x9b, 0xba, 0xd7, 0x6d, 0xd0, 0x44, 0x92, 0x0b, 0x0d, 0xe6, 0xc3,
    0x19, 0x4f, 0xe1, 0xc1,
];

pub fn notary_balance_of(account: &[u8; 20]) -> i64 {
    let args = [StackValue::ByteString(account.to_vec())];
    call_native_read_only(&NOTARY_HASH, "balanceOf", &args)
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn notary_expiration_of(account: &[u8; 20]) -> u32 {
    let args = [StackValue::ByteString(account.to_vec())];
    call_native_read_only(&NOTARY_HASH, "expirationOf", &args)
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn notary_get_max_not_valid_before_delta() -> u32 {
    call_native_read_only(&NOTARY_HASH, "getMaxNotValidBeforeDelta", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn notary_lock_deposit_until(account: &[u8; 20], till: u32) -> bool {
    let args = [
        StackValue::ByteString(account.to_vec()),
        StackValue::Integer(i64::from(till)),
    ];
    call_native(&NOTARY_HASH, "lockDepositUntil", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn notary_withdraw(from: &[u8; 20], to: &[u8; 20]) -> bool {
    let args = [
        StackValue::ByteString(from.to_vec()),
        StackValue::ByteString(to.to_vec()),
    ];
    call_native(&NOTARY_HASH, "withdraw", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn notary_verify(signature: &[u8]) -> bool {
    let args = [StackValue::ByteString(signature.to_vec())];
    call_native_read_only(&NOTARY_HASH, "verify", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn notary_set_max_not_valid_before_delta(value: u32) {
    let args = [StackValue::Integer(i64::from(value))];
    let _ = call_native(&NOTARY_HASH, "setMaxNotValidBeforeDelta", &args);
}
