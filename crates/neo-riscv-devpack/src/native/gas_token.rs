use alloc::string::String;

use neo_riscv_abi::StackValue;

use super::{
    call_native, call_native_read_only, stack_item_as_bool, stack_item_as_i64,
    stack_item_as_string, stack_item_as_u8,
};

// GasToken native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const GAS_TOKEN_HASH: [u8; 20] = [
    0xcf, 0x76, 0xe2, 0x8b, 0xd0, 0x06, 0x2c, 0x4a, 0x47, 0x8e, 0xe3, 0x55, 0x61, 0x01, 0x13, 0x19,
    0xf3, 0xcf, 0xa4, 0xd2,
];

pub fn gas_balance_of(account: &[u8; 20]) -> i64 {
    let args = [StackValue::ByteString(account.to_vec())];
    call_native_read_only(&GAS_TOKEN_HASH, "balanceOf", &args)
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn gas_transfer(from: &[u8; 20], to: &[u8; 20], amount: i64) -> bool {
    let args = [
        StackValue::ByteString(from.to_vec()),
        StackValue::ByteString(to.to_vec()),
        StackValue::Integer(amount),
        StackValue::Null,
    ];
    call_native(&GAS_TOKEN_HASH, "transfer", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn gas_symbol() -> String {
    const DEFAULT: &str = "GAS";
    call_native_read_only(&GAS_TOKEN_HASH, "symbol", &[])
        .and_then(|v| stack_item_as_string(&v))
        .unwrap_or_else(|| String::from(DEFAULT))
}

pub fn gas_decimals() -> u8 {
    call_native_read_only(&GAS_TOKEN_HASH, "decimals", &[])
        .and_then(|v| stack_item_as_u8(&v))
        .unwrap_or(8)
}

pub fn gas_total_supply() -> i64 {
    call_native_read_only(&GAS_TOKEN_HASH, "totalSupply", &[])
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}
