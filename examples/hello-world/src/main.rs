#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

include!("../../support/polkavm_entry.rs");

#[no_mangle]
pub extern "C" fn invoke() -> i32 {
    hello_world::invoke_entry()
}
