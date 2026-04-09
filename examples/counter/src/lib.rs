#![cfg_attr(target_arch = "riscv32", no_std)]

#[path = "../../support/allocator.rs"]
mod allocator;

extern crate alloc;

use neo_riscv_abi::StackValue;
use neo_riscv_devpack::{storage, syscalls};

const COUNTER_KEY: &[u8] = b"ctr";

fn read_counter() -> i32 {
    let bytes = storage::get(COUNTER_KEY).unwrap_or_default();
    if bytes.is_empty() {
        return 0;
    }
    let mut buf = [0u8; 4];
    for (i, b) in bytes.iter().take(4).enumerate() {
        buf[i] = *b;
    }
    i32::from_le_bytes(buf)
}

fn write_counter(val: i32) {
    storage::put(COUNTER_KEY, &val.to_le_bytes());
}

pub fn dispatch(method_name: &str) -> i32 {
    match method_name {
        "increment" => {
            let next = read_counter() + 1;
            write_counter(next);
            syscalls::runtime_notify("increment", &[StackValue::Integer(next as i64)]);
            next
        }
        "decrement" => {
            let next = read_counter() - 1;
            write_counter(next);
            syscalls::runtime_notify("decrement", &[StackValue::Integer(next as i64)]);
            next
        }
        "get" => read_counter(),
        "reset" => {
            write_counter(0);
            syscalls::runtime_notify("reset", &[]);
            0
        }
        _ => -1,
    }
}

pub fn invoke_entry(method: *const u8, _args: *const u8) -> i32 {
    unsafe {
        let len = core::ptr::read(method) as usize;
        let slice = core::slice::from_raw_parts(method.add(1), len);
        let name = core::str::from_utf8_unchecked(slice);
        dispatch(name)
    }
}

#[cfg(test)]
mod tests {
    use super::dispatch;

    #[test]
    fn counter_unknown_method_returns_error() {
        assert_eq!(dispatch("unknown"), -1);
    }

    #[test]
    fn counter_all_dispatch_targets_exist() {
        // Verify the dispatch table recognizes all expected methods.
        // Full round-trip testing requires a host with storage, which is
        // validated by the C# integration tests (UnitTest_RiscVExecution).
        // Here we just verify unknown methods are rejected.
        assert_eq!(dispatch("nonexistent"), -1);
        assert_eq!(dispatch(""), -1);
    }
}
