use alloc::vec::Vec;

use neo_riscv_abi::StackValue;

use super::{call_native_read_only, stack_item_as_fixed_bytes, stack_item_into_items};

// RoleManagement native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const ROLE_MANAGEMENT_HASH: [u8; 20] = [
    0xe2, 0x95, 0xe3, 0x91, 0x54, 0x4c, 0x17, 0x8a, 0xd9, 0x4f, 0x03, 0xec, 0x4d, 0xcd, 0xff, 0x78,
    0x53, 0x4e, 0xcf, 0x49,
];

pub fn role_get_designated_by_role(role: u8, index: u32) -> Vec<[u8; 33]> {
    let args = [
        StackValue::Integer(i64::from(role)),
        StackValue::Integer(i64::from(index)),
    ];
    let value = match call_native_read_only(&ROLE_MANAGEMENT_HASH, "getDesignatedByRole", &args) {
        Some(value) => value,
        None => return Vec::new(),
    };
    let items = match stack_item_into_items(value) {
        Some(items) => items,
        None => return Vec::new(),
    };
    items
        .iter()
        .filter_map(stack_item_as_fixed_bytes::<33>)
        .collect()
}
