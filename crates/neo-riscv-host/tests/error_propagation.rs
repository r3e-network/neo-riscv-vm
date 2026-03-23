use neo_riscv_host::PolkaVmRuntime;

#[test]
fn error_propagation_guest_to_host() {
    // Test error propagates from guest to host
    let invalid_script = vec![0xFF]; // Invalid opcode
    let result = neo_riscv_host::execute_script(&invalid_script);
    assert!(result.is_err());
}

#[test]
fn error_propagation_result_types() {
    // Test Result types propagate correctly
    let runtime = PolkaVmRuntime::new();
    assert!(runtime.is_ok());
}
