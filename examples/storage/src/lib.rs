#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

static mut STORED_VALUE: i32 = 0;

pub fn dispatch(method: &str) -> i32 {
    unsafe {
        match method {
            "put" => {
                STORED_VALUE = 1;
                1
            }
            "get" => STORED_VALUE,
            "delete" => {
                STORED_VALUE = 0;
                0
            }
            _ => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn invoke(_method: *const u8, _args: *const u8) -> i32 {
    dispatch("get")
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
    fn storage_dispatch_cycle() {
        assert_eq!(dispatch("delete"), 0);
        assert_eq!(dispatch("get"), 0);
        assert_eq!(dispatch("put"), 1);
        assert_eq!(dispatch("get"), 1);
        assert_eq!(dispatch("delete"), 0);
        assert_eq!(dispatch("unknown"), -1);
    }
}
