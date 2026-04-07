#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

extern crate alloc;

use neo_riscv_devpack::{storage, syscalls};

const COUNTER_KEY: &[u8] = b"counter";

/// Dispatch method for the storage-based counter contract.
/// Returns the counter value, or -1 for unknown methods.
pub fn dispatch(method: &str) -> i32 {
    match method {
        "get" => {
            let bytes = storage::get(COUNTER_KEY).unwrap_or_default();
            if bytes.is_empty() {
                return 0;
            }
            // Interpret first 4 bytes as LE i32
            let mut buf = [0u8; 4];
            for (i, b) in bytes.iter().take(4).enumerate() {
                buf[i] = *b;
            }
            i32::from_le_bytes(buf)
        }
        "put" => {
            let val: i32 = 1;
            storage::put(COUNTER_KEY, &val.to_le_bytes());
            syscalls::runtime_notify("put", &[neo_riscv_abi::StackValue::Integer(1)]);
            1
        }
        "increment" => {
            let current = dispatch("get");
            let next = current + 1;
            storage::put(COUNTER_KEY, &next.to_le_bytes());
            syscalls::runtime_notify(
                "increment",
                &[neo_riscv_abi::StackValue::Integer(next as i64)],
            );
            next
        }
        "delete" => {
            storage::delete(COUNTER_KEY);
            syscalls::runtime_notify("delete", &[]);
            0
        }
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn invoke(method: *const u8, _args: *const u8) -> i32 {
    unsafe {
        let len = core::ptr::read(method) as usize;
        let slice = core::slice::from_raw_parts(method.add(1), len);
        let name = core::str::from_utf8_unchecked(slice);
        dispatch(name)
    }
}

#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[cfg(test)]
mod tests {
    use super::dispatch;

    #[test]
    fn storage_counter_logic() {
        // This test exercises the dispatch function logic.
        // In a real RISC-V environment, the host provides storage.
        // Here we test that unknown methods return -1.
        assert_eq!(dispatch("unknown"), -1);
    }
}
