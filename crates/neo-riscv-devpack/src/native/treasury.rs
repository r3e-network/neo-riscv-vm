use neo_riscv_abi::StackValue;

use super::{call_native, call_native_read_only, stack_item_as_bool};

// Treasury native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const TREASURY_HASH: [u8; 20] = [
    0xc1, 0x3a, 0x56, 0xc9, 0x83, 0x53, 0xa7, 0xea, 0x6a, 0x32, 0x4d, 0x9a, 0x83, 0x5d, 0x1b, 0x5b,
    0xf2, 0x26, 0x63, 0x15,
];

pub fn treasury_verify() -> bool {
    call_native_read_only(&TREASURY_HASH, "verify", &[])
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn treasury_on_nep17_payment(from: &[u8; 20], amount: i64, data: StackValue) {
    let args = [
        StackValue::ByteString(from.to_vec()),
        StackValue::Integer(amount),
        data,
    ];
    let _ = call_native(&TREASURY_HASH, "onNEP17Payment", &args);
}

pub fn treasury_on_nep11_payment(from: &[u8; 20], amount: i64, token_id: &[u8], data: StackValue) {
    let args = [
        StackValue::ByteString(from.to_vec()),
        StackValue::Integer(amount),
        StackValue::ByteString(token_id.to_vec()),
        data,
    ];
    let _ = call_native(&TREASURY_HASH, "onNEP11Payment", &args);
}
