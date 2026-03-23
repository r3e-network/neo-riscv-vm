#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

static mut TOTAL_SUPPLY: i32 = 1_000_000;

pub fn dispatch(method: &str) -> i32 {
    unsafe {
        match method {
            "totalSupply" => TOTAL_SUPPLY,
            "symbol" => 0,
            "decimals" => 8,
            "transfer" => 1,
            _ => -1,
        }
    }
}

#[no_mangle]
pub extern "C" fn invoke() -> i32 {
    dispatch("totalSupply")
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
    fn nep17_metadata_dispatches() {
        assert_eq!(dispatch("totalSupply"), 1_000_000);
        assert_eq!(dispatch("decimals"), 8);
        assert_eq!(dispatch("transfer"), 1);
        assert_eq!(dispatch("unknown"), -1);
    }
}
