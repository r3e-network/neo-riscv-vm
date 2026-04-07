use neo_riscv_rt::{Context, StackValue};

/// Helper: create an empty context with no initial stack.
fn empty_ctx() -> Context {
    Context::from_abi_stack(vec![])
}

// ---------------------------------------------------------------
// 1. push/pop integer
// ---------------------------------------------------------------

#[test]
fn push_pop_integer() {
    let mut ctx = empty_ctx();
    ctx.push_int(42);
    let val = ctx.pop();
    assert_eq!(val, StackValue::Integer(42));
    assert!(ctx.stack.is_empty());
}

// ---------------------------------------------------------------
// 2. init_slot loads args from stack
// ---------------------------------------------------------------

#[test]
fn init_slot_loads_args_from_stack() {
    let mut ctx = empty_ctx();
    ctx.push_int(10);
    ctx.push_int(20);

    // 1 local slot, 2 argument slots
    ctx.init_slot(1, 2);

    // Arguments are reversed from pop order: arg[0]=10, arg[1]=20
    assert_eq!(ctx.args.len(), 2);
    assert_eq!(ctx.args[0], StackValue::Integer(10));
    assert_eq!(ctx.args[1], StackValue::Integer(20));

    // Stack should be empty after popping 2 items
    assert!(ctx.stack.is_empty());

    // Local slot initialized to Null
    assert_eq!(ctx.locals.len(), 1);
    assert_eq!(ctx.locals[0], StackValue::Null);
}

// ---------------------------------------------------------------
// 3. add integers
// ---------------------------------------------------------------

#[test]
fn add_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(3);
    ctx.push_int(4);
    ctx.add();
    assert_eq!(ctx.pop(), StackValue::Integer(7));
}

// ---------------------------------------------------------------
// 4. sub integers
// ---------------------------------------------------------------

#[test]
fn sub_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(10);
    ctx.push_int(3);
    ctx.sub();
    assert_eq!(ctx.pop(), StackValue::Integer(7));
}

// ---------------------------------------------------------------
// 5. mul integers
// ---------------------------------------------------------------

#[test]
fn mul_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(3);
    ctx.push_int(4);
    ctx.mul();
    assert_eq!(ctx.pop(), StackValue::Integer(12));
}

// ---------------------------------------------------------------
// 6. equal integers
// ---------------------------------------------------------------

#[test]
fn equal_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(5);
    ctx.push_int(5);
    ctx.equal();
    assert_eq!(ctx.pop(), StackValue::Boolean(true));
}

#[test]
fn not_equal_integers() {
    let mut ctx = empty_ctx();
    ctx.push_int(5);
    ctx.push_int(6);
    ctx.equal();
    assert_eq!(ctx.pop(), StackValue::Boolean(false));
}

// ---------------------------------------------------------------
// 7. local variable store/load
// ---------------------------------------------------------------

#[test]
fn local_variable_store_load() {
    let mut ctx = empty_ctx();
    ctx.init_slot(2, 0); // 2 locals, 0 args

    ctx.push_int(99);
    ctx.store_local(0);

    ctx.push_int(100);
    ctx.store_local(1);

    ctx.load_local(0);
    assert_eq!(ctx.pop(), StackValue::Integer(99));

    ctx.load_local(1);
    assert_eq!(ctx.pop(), StackValue::Integer(100));
}

// ---------------------------------------------------------------
// 8. dup and swap
// ---------------------------------------------------------------

#[test]
fn dup_and_swap() {
    let mut ctx = empty_ctx();
    ctx.push_int(1);
    ctx.push_int(2);

    // dup should duplicate the top value (2)
    ctx.dup();
    assert_eq!(ctx.stack.len(), 3);
    assert_eq!(ctx.pop(), StackValue::Integer(2)); // duplicated top

    // stack is now [1, 2]; swap should put 1 on top
    ctx.swap();
    assert_eq!(ctx.pop(), StackValue::Integer(1));
    assert_eq!(ctx.pop(), StackValue::Integer(2));
}

// ---------------------------------------------------------------
// 9. push null
// ---------------------------------------------------------------

#[test]
fn push_null() {
    let mut ctx = empty_ctx();
    ctx.push_null();
    let val = ctx.pop();
    assert_eq!(val, StackValue::Null);
}

// ---------------------------------------------------------------
// 10. push bool
// ---------------------------------------------------------------

#[test]
fn push_bool_true_and_false() {
    let mut ctx = empty_ctx();
    ctx.push_bool(true);
    ctx.push_bool(false);

    assert_eq!(ctx.pop(), StackValue::Boolean(false));
    assert_eq!(ctx.pop(), StackValue::Boolean(true));
}

// ---------------------------------------------------------------
// 11. static fields
// ---------------------------------------------------------------

#[test]
fn static_fields_store_load() {
    let mut ctx = empty_ctx();
    ctx.push_int(42);
    ctx.store_static(0);

    ctx.load_static(0);
    assert_eq!(ctx.pop(), StackValue::Integer(42));
}

#[test]
fn static_fields_auto_extend() {
    let mut ctx = empty_ctx();
    // Store into a high index -- should auto-extend
    ctx.push_int(77);
    ctx.store_static(5);
    assert_eq!(ctx.static_fields.len(), 6);

    ctx.load_static(5);
    assert_eq!(ctx.pop(), StackValue::Integer(77));

    // Loading an unset intermediate slot should yield Null
    ctx.load_static(2);
    assert_eq!(ctx.pop(), StackValue::Null);
}

// ---------------------------------------------------------------
// 12. push bytes
// ---------------------------------------------------------------

#[test]
fn push_bytes() {
    let mut ctx = empty_ctx();
    ctx.push_bytes(&[1, 2, 3]);
    let val = ctx.pop();
    assert_eq!(val, StackValue::ByteString(vec![1, 2, 3]));
}
