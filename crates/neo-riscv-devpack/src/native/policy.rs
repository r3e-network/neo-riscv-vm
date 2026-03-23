use neo_riscv_abi::StackValue;

use super::{call_native_read_only, stack_item_as_bool, stack_item_as_i64, stack_item_as_u32};

// PolicyContract native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const POLICY_CONTRACT_HASH: [u8; 20] = [
    0x7b, 0xc6, 0x81, 0xc0, 0xa1, 0xf7, 0x1d, 0x54, 0x34, 0x57, 0xb6, 0x8b, 0xba, 0x8d, 0x5f, 0x9f,
    0xdd, 0x4e, 0x5e, 0xcc,
];

pub fn policy_get_fee_per_byte() -> i64 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getFeePerByte", &[])
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn policy_get_exec_fee_factor() -> u32 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getExecFeeFactor", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn policy_get_storage_price() -> u32 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getStoragePrice", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn policy_is_blocked(account: &[u8; 20]) -> bool {
    let args = [StackValue::ByteString(account.to_vec())];
    call_native_read_only(&POLICY_CONTRACT_HASH, "isBlocked", &args)
        .and_then(|v| stack_item_as_bool(&v))
        .unwrap_or(false)
}

pub fn policy_get_attribute_fee(attribute_type: u8) -> u32 {
    let args = [StackValue::Integer(i64::from(attribute_type))];
    call_native_read_only(&POLICY_CONTRACT_HASH, "getAttributeFee", &args)
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn policy_get_milliseconds_per_block() -> u32 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getMillisecondsPerBlock", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(15000)
}

pub fn policy_get_max_valid_until_block_increment() -> u32 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getMaxValidUntilBlockIncrement", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(5760)
}

pub fn policy_get_max_traceable_blocks() -> u32 {
    call_native_read_only(&POLICY_CONTRACT_HASH, "getMaxTraceableBlocks", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(2_102_400)
}
