use neo_riscv_abi::{StackValue, VmState};
use neo_riscv_host::{execute_script_with_context, PolkaVmRuntime, RuntimeContext};

#[test]
fn runtime_initializes_for_gas_validation() {
    let _runtime = PolkaVmRuntime::new().expect("runtime init");
}

#[test]
fn opcode_fee_consumption_matches_configured_exec_fee_factor() {
    let result = execute_script_with_context(
        &[0x11, 0x40], // PUSH1, RET
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 100_000,
            exec_fee_factor_pico: 10_000,
        },
    )
    .expect("script should execute with fee accounting");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
    assert_eq!(
        result.fee_consumed_pico, 10_000,
        "one PUSH1 should consume one fee unit at the configured factor",
    );
}
