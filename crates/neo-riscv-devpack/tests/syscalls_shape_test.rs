use neo_riscv_abi::StackValue;
use neo_riscv_devpack::syscalls::{build_contract_call_stack, CALL_FLAGS_ALL};

#[test]
fn build_contract_call_stack_matches_bridge_tail_order() {
    let args = [StackValue::Integer(7), StackValue::Boolean(true)];

    let stack = build_contract_call_stack(
        &[0x55; 20],
        "transfer",
        CALL_FLAGS_ALL,
        &args,
    );

    assert_eq!(
        stack,
        vec![
            StackValue::Array(args.to_vec()),
            StackValue::Integer(i64::from(CALL_FLAGS_ALL)),
            StackValue::ByteString(b"transfer".to_vec()),
            StackValue::ByteString(vec![0x55; 20]),
        ]
    );
}

#[test]
fn build_contract_call_stack_preserves_custom_flags_and_empty_args() {
    let stack = build_contract_call_stack(&[0x11; 20], "balanceOf", 0x05, &[]);

    assert_eq!(
        stack,
        vec![
            StackValue::Array(Vec::new()),
            StackValue::Integer(5),
            StackValue::ByteString(b"balanceOf".to_vec()),
            StackValue::ByteString(vec![0x11; 20]),
        ]
    );
}
