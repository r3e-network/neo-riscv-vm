#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn invoke() -> i32 {
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
