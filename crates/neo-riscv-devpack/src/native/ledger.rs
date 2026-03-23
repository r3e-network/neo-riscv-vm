use alloc::vec::Vec;

use neo_riscv_abi::StackValue;

use super::{
    call_native_read_only, stack_item_as_fixed_bytes, stack_item_as_i64, stack_item_as_u32,
    std_lib::stdlib_serialize_stack_item,
};

// LedgerContract native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const LEDGER_CONTRACT_HASH: [u8; 20] = [
    0xbe, 0xf2, 0x04, 0x31, 0x40, 0x36, 0x2a, 0x77, 0xc1, 0x50, 0x99, 0xc7, 0xe6, 0x4c, 0x12, 0xf7,
    0x00, 0xb6, 0x65, 0xda,
];

// Ledger native contract bindings
pub fn ledger_get_block(index: u32) -> Option<Vec<u8>> {
    let args = [StackValue::Integer(i64::from(index))];
    call_native_read_only(&LEDGER_CONTRACT_HASH, "getBlock", &args)
        .and_then(|value| stdlib_serialize_stack_item(&value))
}

pub fn ledger_get_transaction(hash: &[u8; 32]) -> Option<Vec<u8>> {
    let args = [StackValue::ByteString(hash.to_vec())];
    call_native_read_only(&LEDGER_CONTRACT_HASH, "getTransaction", &args)
        .and_then(|value| stdlib_serialize_stack_item(&value))
}

pub fn ledger_current_index() -> u32 {
    call_native_read_only(&LEDGER_CONTRACT_HASH, "currentIndex", &[])
        .and_then(|v| stack_item_as_u32(&v))
        .unwrap_or(0)
}

pub fn ledger_get_transaction_height(hash: &[u8; 32]) -> Option<u32> {
    let args = [StackValue::ByteString(hash.to_vec())];
    let height = call_native_read_only(&LEDGER_CONTRACT_HASH, "getTransactionHeight", &args)
        .and_then(|v| stack_item_as_i64(&v))?;
    if height < 0 {
        return None;
    }
    u32::try_from(height).ok()
}

pub fn ledger_current_hash() -> [u8; 32] {
    call_native_read_only(&LEDGER_CONTRACT_HASH, "currentHash", &[])
        .and_then(|v| stack_item_as_fixed_bytes::<32>(&v))
        .unwrap_or([0; 32])
}
