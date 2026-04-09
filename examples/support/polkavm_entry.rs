#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub extern "C" fn _start() {}

#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub extern "C" fn main() {}

#[cfg(not(target_arch = "riscv32"))]
fn main() {}

#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
