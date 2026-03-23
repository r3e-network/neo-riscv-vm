use crate::{api_ids, ffi};
use alloc::{vec, vec::Vec};
use neo_riscv_abi::StackValue;

pub fn get(key: &[u8]) -> Option<Vec<u8>> {
    let stack = vec![StackValue::ByteString(key.to_vec())];
    let result = ffi::invoke_host_call(api_ids::STORAGE_GET, &stack).ok()?;
    match result.first()? {
        StackValue::ByteString(data) => Some(data.clone()),
        _ => None,
    }
}

pub fn put(key: &[u8], value: &[u8]) {
    let stack = vec![
        StackValue::ByteString(key.to_vec()),
        StackValue::ByteString(value.to_vec()),
    ];
    let _ = ffi::invoke_host_call(api_ids::STORAGE_PUT, &stack);
}

pub fn delete(key: &[u8]) {
    let stack = vec![StackValue::ByteString(key.to_vec())];
    let _ = ffi::invoke_host_call(api_ids::STORAGE_DELETE, &stack);
}
