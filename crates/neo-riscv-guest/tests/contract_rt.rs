use neo_riscv_guest::contract_rt::{Context, StackValue};
use neo_riscv_guest::runtime::ops;

#[test]
fn contract_runtime_context_executes_shared_vm_runtime_ops() {
    let mut context = Context::from_abi_stack(vec![]);

    context.push_int(10);
    context.push_int(3);
    ops::arithmetic::sub(&mut context);

    assert_eq!(context.pop(), StackValue::Integer(7));
}
