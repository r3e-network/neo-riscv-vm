use alloc::{vec, vec::Vec};
use neo_riscv_abi::StackValue;
use crate::{api_ids, ffi};

pub const CALL_FLAGS_ALL: u8 = 0x0f;

// System.Contract
pub fn contract_call(hash: &[u8], method: &str, args: &[StackValue]) -> StackValue {
    contract_call_with_flags(hash, method, CALL_FLAGS_ALL, args)
}

#[must_use]
pub fn build_contract_call_stack(
    hash: &[u8],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> Vec<StackValue> {
    vec![
        StackValue::Array(args.to_vec()),
        StackValue::Integer(i64::from(call_flags)),
        StackValue::ByteString(method.as_bytes().to_vec()),
        StackValue::ByteString(hash.to_vec()),
    ]
}

pub fn contract_call_with_flags(
    hash: &[u8],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> StackValue {
    let stack = build_contract_call_stack(hash, method, call_flags, args);
    ffi::invoke_host_call(api_ids::CONTRACT_CALL, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

pub fn contract_create(nef: &[u8], manifest: &[u8]) -> StackValue {
    let stack = vec![
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::CONTRACT_CREATE, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

pub fn contract_update(nef: &[u8], manifest: &[u8]) {
    let stack = vec![
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    let _ = ffi::invoke_host_call(api_ids::CONTRACT_UPDATE, &stack);
}

// System.Runtime
pub fn runtime_notify(event: &str, state: &[StackValue]) {
    let mut stack = vec![StackValue::ByteString(event.as_bytes().to_vec())];
    stack.extend_from_slice(state);
    let _ = ffi::invoke_host_call(api_ids::RUNTIME_NOTIFY, &stack);
}

pub fn runtime_log(message: &str) {
    let stack = vec![StackValue::ByteString(message.as_bytes().to_vec())];
    let _ = ffi::invoke_host_call(api_ids::RUNTIME_LOG, &stack);
}

pub fn runtime_check_witness(hash: &[u8]) -> bool {
    let stack = vec![StackValue::ByteString(hash.to_vec())];
    ffi::invoke_host_call(api_ids::RUNTIME_CHECK_WITNESS, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}

// System.Crypto
pub fn crypto_verify_signature(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool {
    let stack = vec![
        StackValue::ByteString(message.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
        StackValue::ByteString(signature.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::CRYPTO_VERIFY_SIGNATURE, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}
