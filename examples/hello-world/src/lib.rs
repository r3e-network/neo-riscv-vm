#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

pub fn dispatch() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn invoke() -> i32 {
    dispatch()
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
    fn hello_world_dispatch_succeeds() {
        assert_eq!(dispatch(), 0);
    }
}
