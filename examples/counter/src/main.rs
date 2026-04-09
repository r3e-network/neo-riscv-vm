#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

include!("../../support/polkavm_entry.rs");

#[no_mangle]
pub extern "C" fn invoke(method: *const u8, args: *const u8) -> i32 {
    counter::invoke_entry(method, args)
}
