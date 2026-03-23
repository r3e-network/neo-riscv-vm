#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

use core::ptr;

static mut COUNTER: i32 = 0;

pub fn dispatch(method_name: &str) -> i32 {
    unsafe {
        match method_name {
            "increment" => {
                COUNTER += 1;
                COUNTER
            }
            "get" => COUNTER,
            "reset" => {
                COUNTER = 0;
                0
            }
            _ => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn invoke(method: *const u8, _args: *const u8) -> i32 {
    unsafe { dispatch(read_string(method)) }
}

unsafe fn read_string(ptr: *const u8) -> &'static str {
    let len = ptr::read(ptr) as usize;
    let slice = core::slice::from_raw_parts(ptr.add(1), len);
    core::str::from_utf8_unchecked(slice)
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
    fn counter_dispatch_round_trip() {
        assert_eq!(dispatch("reset"), 0);
        assert_eq!(dispatch("increment"), 1);
        assert_eq!(dispatch("increment"), 2);
        assert_eq!(dispatch("get"), 2);
        assert_eq!(dispatch("reset"), 0);
        assert_eq!(dispatch("unknown"), -1);
    }
}
