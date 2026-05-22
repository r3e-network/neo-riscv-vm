use neo_riscv_guest::contract_rt::{Context, StackValue};
use neo_riscv_guest::{semantics::runtime, VmState};

fn empty_ctx() -> Context {
    Context::from_abi_stack(vec![])
}

#[test]
fn push_pop_integer() {
    let mut ctx = empty_ctx();
    ctx.push_int(42);
    let val = ctx.pop();
    assert_eq!(val, StackValue::Integer(42));
    assert!(ctx.stack.is_empty());
}

#[test]
fn init_slot_loads_args_from_stack() {
    let mut ctx = empty_ctx();
    ctx.push_int(10);
    ctx.push_int(20);

    ctx.init_slot(1, 2);

    assert_eq!(ctx.args.len(), 2);
    assert_eq!(ctx.args[0], StackValue::Integer(10));
    assert_eq!(ctx.args[1], StackValue::Integer(20));
    assert!(ctx.stack.is_empty());
    assert_eq!(ctx.locals.len(), 1);
    assert_eq!(ctx.locals[0], StackValue::Null);
}

#[test]
fn add_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(3);
    ctx.push_int(4);
    runtime::arithmetic::add(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Integer(7));
}

#[test]
fn sub_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(10);
    ctx.push_int(3);
    runtime::arithmetic::sub(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Integer(7));
}

#[test]
fn mul_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(3);
    ctx.push_int(4);
    runtime::arithmetic::mul(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Integer(12));
}

#[test]
fn equal_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(5);
    ctx.push_int(5);
    runtime::comparison::equal(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Boolean(true));
}

#[test]
fn not_equal_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(5);
    ctx.push_int(6);
    runtime::comparison::equal(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Boolean(false));
}

#[test]
fn local_variable_store_load() {
    let mut ctx = empty_ctx();
    ctx.init_slot(2, 0);

    ctx.push_int(99);
    ctx.store_local(0);

    ctx.push_int(100);
    ctx.store_local(1);

    ctx.load_local(0);
    assert_eq!(ctx.pop(), StackValue::Integer(99));

    ctx.load_local(1);
    assert_eq!(ctx.pop(), StackValue::Integer(100));
}

#[test]
fn dup_and_swap() {
    let mut ctx = empty_ctx();
    ctx.push_int(1);
    ctx.push_int(2);

    runtime::stack::dup(&mut ctx);
    assert_eq!(ctx.stack.len(), 3);
    assert_eq!(ctx.pop(), StackValue::Integer(2));

    runtime::stack::swap(&mut ctx);
    assert_eq!(ctx.pop(), StackValue::Integer(1));
    assert_eq!(ctx.pop(), StackValue::Integer(2));
}

#[test]
fn push_null() {
    let mut ctx = empty_ctx();
    ctx.push_null();
    let val = ctx.pop();
    assert_eq!(val, StackValue::Null);
}

#[test]
fn push_bool_true_and_false() {
    let mut ctx = empty_ctx();
    ctx.push_bool(true);
    ctx.push_bool(false);

    assert_eq!(ctx.pop(), StackValue::Boolean(false));
    assert_eq!(ctx.pop(), StackValue::Boolean(true));
}

#[test]
fn static_fields_store_load() {
    let mut ctx = empty_ctx();
    ctx.init_sslot(1);
    ctx.push_int(42);
    ctx.store_static(0);

    ctx.load_static(0);
    assert_eq!(ctx.pop(), StackValue::Integer(42));
}

#[test]
fn static_fields_reject_uninitialized_or_out_of_range_access() {
    let mut ctx = empty_ctx();
    ctx.push_int(77);
    ctx.store_static(5);
    assert_eq!(ctx.state, VmState::Fault);
    assert_eq!(
        ctx.fault_message.as_deref(),
        Some("invalid static field index")
    );

    let mut ctx = empty_ctx();
    ctx.init_sslot(1);
    ctx.load_static(1);
    assert_eq!(ctx.state, VmState::Fault);
    assert_eq!(
        ctx.fault_message.as_deref(),
        Some("invalid static field index")
    );

    let mut ctx = empty_ctx();
    ctx.init_sslot(3);
    ctx.load_static(2);
    assert_eq!(ctx.state, VmState::Halt);
    assert_eq!(ctx.pop(), StackValue::Null);
}

#[test]
fn push_bytes() {
    let mut ctx = empty_ctx();
    ctx.push_bytes(&[1, 2, 3]);
    let val = ctx.pop();
    assert_eq!(val, StackValue::ByteString(vec![1, 2, 3]));
}
