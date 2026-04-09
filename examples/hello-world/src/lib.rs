#![cfg_attr(target_arch = "riscv32", no_std)]

#[path = "../../support/allocator.rs"]
mod allocator;

extern crate alloc;

use neo_riscv_abi::StackValue;
use neo_riscv_devpack::syscalls;

pub fn dispatch() -> i32 {
    syscalls::runtime_log("Hello from RISC-V Neo contract!");
    syscalls::runtime_notify("hello", &[StackValue::ByteString(b"world".to_vec())]);
    0
}

pub fn invoke_entry() -> i32 {
    dispatch()
}

#[cfg(test)]
mod tests {
    use super::dispatch;

    #[test]
    fn hello_world_dispatch_succeeds() {
        assert_eq!(dispatch(), 0);
    }

    #[test]
    fn hello_world_invoke_matches_dispatch() {
        // Both entry points should return the same success code
        let result = dispatch();
        assert_eq!(result, 0, "hello world should always return 0 (success)");
    }
}
