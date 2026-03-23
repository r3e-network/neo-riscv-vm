#![no_std]
#![no_main]

use core::ptr;

static mut COUNTER: i32 = 0;

#[no_mangle]
pub extern "C" fn invoke(method: *const u8, _args: *const u8) -> i32 {
    unsafe {
        let method_name = read_string(method);

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
            _ => -1
        }
    }
}

unsafe fn read_string(ptr: *const u8) -> &'static str {
    let len = ptr::read(ptr) as usize;
    let slice = core::slice::from_raw_parts(ptr.add(1), len);
    core::str::from_utf8_unchecked(slice)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
