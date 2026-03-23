use neo_riscv_abi::StackValue;
use neo_riscv_devpack::storage::{delete, get, put};
use neo_riscv_devpack::syscalls::{
    contract_call, contract_call_with_flags, crypto_verify_signature, runtime_check_witness,
    runtime_log, runtime_notify,
};

#[no_mangle]
pub extern "C" fn host_call(
    _api: u32,
    _ip: u32,
    _stack_ptr: usize,
    _stack_len: usize,
    _result_ptr: usize,
    _result_cap: usize,
) -> usize {
    0
}

#[test]
fn storage_get_returns_none() {
    let result = get(b"key");
    assert_eq!(result, None);
}

#[test]
fn storage_put_does_not_panic() {
    put(b"key", b"value");
}

#[test]
fn storage_delete_does_not_panic() {
    delete(b"key");
}

#[test]
fn contract_call_returns_null() {
    let result = contract_call(b"12345678901234567890", "balanceOf", &[]);
    assert_eq!(result, StackValue::Null);
}

#[test]
fn contract_call_with_flags_returns_null() {
    let result = contract_call_with_flags(
        b"12345678901234567890",
        "balanceOf",
        0x05,
        &[StackValue::Integer(1)],
    );
    assert_eq!(result, StackValue::Null);
}

#[test]
fn runtime_notify_does_not_panic() {
    runtime_notify("event", &[]);
}

#[test]
fn runtime_log_does_not_panic() {
    runtime_log("message");
}

#[test]
fn runtime_check_witness_returns_false() {
    let result = runtime_check_witness(b"hash");
    assert!(!result);
}

#[test]
fn crypto_verify_signature_returns_false() {
    let result = crypto_verify_signature(b"msg", b"pub", b"sig");
    assert!(!result);
}
