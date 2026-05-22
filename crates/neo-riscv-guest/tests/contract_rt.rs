use neo_riscv_guest::contract_rt::{Context, StackValue};
use neo_riscv_guest::{semantics::runtime, VmState};

#[test]
fn contract_runtime_context_executes_shared_vm_runtime_ops() {
    let mut context = Context::from_abi_stack(vec![]);

    context.push_int(10);
    context.push_int(3);
    runtime::arithmetic::sub(&mut context);

    assert_eq!(context.pop(), StackValue::Integer(7));
}

#[test]
fn contract_runtime_syscall_stubs_fault_through_shared_vm_context() {
    let mut context = Context::from_abi_stack(vec![]);

    context.call_token(0);

    assert_eq!(context.state, VmState::Fault);
    assert_eq!(
        context.fault_message.as_deref(),
        Some("CALLT: not yet implemented (requires host bridge)")
    );
}
