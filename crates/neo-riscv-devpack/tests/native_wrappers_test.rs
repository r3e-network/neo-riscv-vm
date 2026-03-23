use core::slice;
use std::sync::{Mutex, OnceLock};

use neo_riscv_abi::{callback_codec, StackValue};
use neo_riscv_devpack::native::{
    contract_management, crypto_lib, gas_token, ledger, neo_token, oracle, policy, role_management,
    std_lib,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Invocation {
    api: u32,
    stack: Vec<StackValue>,
}

#[derive(Default)]
struct HostState {
    invocations: Vec<Invocation>,
    response: Vec<u8>,
}

fn host_state() -> &'static Mutex<HostState> {
    static STATE: OnceLock<Mutex<HostState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HostState::default()))
}

fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn reset_host(response: Result<Vec<StackValue>, String>) {
    let mut state = host_state().lock().unwrap();
    state.invocations.clear();
    state.response = callback_codec::encode_stack_result(&response);
}

fn take_invocations() -> Vec<Invocation> {
    let mut state = host_state().lock().unwrap();
    std::mem::take(&mut state.invocations)
}

#[no_mangle]
pub extern "C" fn host_call(
    api: u32,
    _ip: u32,
    stack_ptr: usize,
    stack_len: usize,
    result_ptr: usize,
    result_cap: usize,
) -> usize {
    let stack_bytes = unsafe { slice::from_raw_parts(stack_ptr as *const u8, stack_len) };
    let stack = callback_codec::decode_stack_result(stack_bytes)
        .expect("guest stack payload should decode")
        .expect("guest stack payload should be ok");

    let mut state = host_state().lock().unwrap();
    state.invocations.push(Invocation { api, stack });

    let result = &state.response;
    if result.is_empty() {
        return 0;
    }

    assert!(
        result.len() <= result_cap,
        "result buffer too small for test payload"
    );

    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), result_ptr as *mut u8, result.len());
    }

    result.len()
}

#[test]
fn gas_balance_of_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Integer(42)]));

    let balance = gas_token::gas_balance_of(&[0x11; 20]);

    assert_eq!(balance, 42);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn neo_balance_of_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Integer(7)]));

    let balance = neo_token::neo_balance_of(&[0x22; 20]);

    assert_eq!(balance, 7);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn policy_fee_wrapper_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Integer(1000)]));

    let fee = policy::policy_get_fee_per_byte();

    assert_eq!(fee, 1000);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn crypto_hash_wrapper_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::ByteString(vec![0xaa; 32])]));

    let hash = crypto_lib::crypto_sha256(b"payload");

    assert_eq!(hash, [0xaa; 32]);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn stdlib_encode_wrapper_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::ByteString(b"aGVsbG8=".to_vec())]));

    let encoded = std_lib::stdlib_base64_encode(b"hello");

    assert_eq!(encoded, b"aGVsbG8=".to_vec());

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn ledger_current_index_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Integer(123)]));

    let index = ledger::ledger_current_index();

    assert_eq!(index, 123);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn role_management_wrapper_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Array(vec![
        StackValue::ByteString(vec![0x33; 33]),
        StackValue::ByteString(vec![0x44; 33]),
    ])]));

    let nodes = role_management::role_get_designated_by_role(4, 9);

    assert_eq!(nodes, vec![[0x33; 33], [0x44; 33]]);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn oracle_request_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(Vec::new()));

    oracle::oracle_request("https://example.com", "$.price", "callback", b"user", 5);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}

#[test]
fn contract_management_wrapper_routes_through_contract_call() {
    let _guard = test_lock().lock().unwrap();
    reset_host(Ok(vec![StackValue::Array(vec![
        StackValue::Integer(1),
        StackValue::Integer(0),
        StackValue::ByteString(vec![0x55; 20]),
    ])]));

    let hash = contract_management::contract_deploy(b"nef", b"manifest");

    assert_eq!(hash, [0x55; 20]);

    let invocations = take_invocations();
    assert_eq!(invocations.len(), 1);
}
