#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn invoke(_method: *const u8, _args: *const u8) -> i32 {
    // Storage operations: put, get, delete
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
