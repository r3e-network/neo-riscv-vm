use alloc::vec::Vec;

use neo_riscv_abi::StackValue;

use super::{call_native_read_only, stack_item_as_bytes, stack_item_as_i64};

// Canonical hash from Neo UnitTests (UT_NativeContract.cs), byte order as used on the VM stack
// (UInt160.ToArray() little-endian).
pub const STD_LIB_HASH: [u8; 20] = [
    0xc0, 0xef, 0x39, 0xce, 0xe0, 0xe4, 0xe9, 0x25, 0xc6, 0xc2, 0xa0, 0x6a, 0x79, 0xe1, 0x44, 0x0d,
    0xd8, 0x6f, 0xce, 0xac,
];

pub(crate) fn stdlib_serialize_stack_item(item: &StackValue) -> Option<Vec<u8>> {
    let args = [item.clone()];
    call_native_read_only(&STD_LIB_HASH, "serialize", &args).and_then(|v| stack_item_as_bytes(&v))
}

pub(crate) fn stdlib_deserialize_stack_item(data: &[u8]) -> Option<StackValue> {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "deserialize", &args)
}

// StdLib native contract bindings
pub fn stdlib_serialize(item: &[u8]) -> Vec<u8> {
    // Interpret `item` as a ByteString StackItem.
    stdlib_serialize_stack_item(&StackValue::ByteString(item.to_vec())).unwrap_or_default()
}

pub fn stdlib_deserialize(data: &[u8]) -> Vec<u8> {
    // Return a stable encoding for non-ByteString results by re-serializing via StdLib.serialize.
    let item = match stdlib_deserialize_stack_item(data) {
        Some(item) => item,
        None => return Vec::new(),
    };
    stdlib_serialize_stack_item(&item).unwrap_or_default()
}

pub fn stdlib_json_serialize(item: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(item.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "jsonSerialize", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}

pub fn stdlib_json_deserialize(json: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(json.to_vec())];
    let item = match call_native_read_only(&STD_LIB_HASH, "jsonDeserialize", &args) {
        Some(item) => item,
        None => return Vec::new(),
    };
    stdlib_serialize_stack_item(&item).unwrap_or_default()
}

pub fn stdlib_base64_encode(data: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "base64Encode", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}

pub fn stdlib_base64_decode(data: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "base64Decode", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}

pub fn stdlib_itoa(value: i64, base: u8) -> Vec<u8> {
    let args = [
        StackValue::Integer(value),
        StackValue::Integer(i64::from(base)),
    ];
    call_native_read_only(&STD_LIB_HASH, "itoa", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}

pub fn stdlib_atoi(value: &[u8], base: u8) -> i64 {
    let args = [
        StackValue::ByteString(value.to_vec()),
        StackValue::Integer(i64::from(base)),
    ];
    call_native_read_only(&STD_LIB_HASH, "atoi", &args)
        .and_then(|v| stack_item_as_i64(&v))
        .unwrap_or(0)
}

pub fn stdlib_base58_encode(data: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "base58Encode", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}

pub fn stdlib_base58_decode(data: &[u8]) -> Vec<u8> {
    let args = [StackValue::ByteString(data.to_vec())];
    call_native_read_only(&STD_LIB_HASH, "base58Decode", &args)
        .and_then(|v| stack_item_as_bytes(&v))
        .unwrap_or_default()
}
