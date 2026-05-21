use neo_riscv_guest::{interpret, StackValue, VmState};

#[test]
fn guest_stack_value_is_the_shared_neo_vm_rs_type() {
    let item = StackValue::Integer(1);
    let shared: neo_vm_rs::StackValue = item;

    assert_eq!(shared, neo_vm_rs::StackValue::Integer(1));
}

#[test]
fn guest_interpreter_is_exposed_from_neo_vm_rs() {
    let result = interpret(&[0x12, 0x13, 0x9e, 0x40]).expect("script should execute");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(5)]);
}

#[test]
fn guest_crate_is_a_facade_not_a_private_interpreter() {
    let manifest = include_str!("../Cargo.toml");
    let lib = include_str!("../src/lib.rs");

    assert!(manifest.contains("neo-vm-rs.workspace = true"));
    assert!(lib.contains("pub use neo_vm_rs"));
}
