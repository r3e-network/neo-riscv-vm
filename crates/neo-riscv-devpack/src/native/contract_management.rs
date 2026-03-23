use alloc::vec::Vec;

use neo_riscv_abi::StackValue;

use super::{
    call_native, call_native_read_only, stack_item_as_fixed_bytes,
    std_lib::stdlib_serialize_stack_item,
};

// ContractManagement native contract bindings
//
// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const CONTRACT_MANAGEMENT_HASH: [u8; 20] = [
    0xfd, 0xa3, 0xfa, 0x43, 0x46, 0xea, 0x53, 0x2a, 0x25, 0x8f, 0xc4, 0x97, 0xdd, 0xad, 0xdb, 0x64,
    0x37, 0xc9, 0xfd, 0xff,
];

pub fn contract_deploy(nef: &[u8], manifest: &[u8]) -> [u8; 20] {
    let args = [
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    let result = match call_native(&CONTRACT_MANAGEMENT_HASH, "deploy", &args) {
        Some(result) => result,
        None => return [0; 20],
    };

    match result {
        StackValue::Array(fields) | StackValue::Struct(fields) if fields.len() >= 3 => {
            stack_item_as_fixed_bytes::<20>(&fields[2]).unwrap_or([0; 20])
        }
        _ => [0; 20],
    }
}

pub fn contract_update(nef: &[u8], manifest: &[u8]) {
    let args = [
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    let _ = call_native(&CONTRACT_MANAGEMENT_HASH, "update", &args);
}

pub fn contract_destroy() {
    let _ = call_native(&CONTRACT_MANAGEMENT_HASH, "destroy", &[]);
}

pub fn contract_get_contract(hash: &[u8; 20]) -> Option<Vec<u8>> {
    let args = [StackValue::ByteString(hash.to_vec())];
    call_native_read_only(&CONTRACT_MANAGEMENT_HASH, "getContract", &args)
        .and_then(|value| stdlib_serialize_stack_item(&value))
}
