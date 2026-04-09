#![cfg_attr(target_arch = "riscv32", no_std)]

#[path = "../../support/allocator.rs"]
mod allocator;

extern crate alloc;

use neo_riscv_abi::StackValue;
use neo_riscv_devpack::{storage, syscalls};

const TOTAL_SUPPLY_KEY: &[u8] = b"totalSupply";
const BALANCE_PREFIX: &[u8] = b"balance:";

const TOTAL_SUPPLY: i64 = 1_000_000_000; // 10 million with 8 decimals
const DECIMALS: i64 = 8;

fn balance_key(account: &[u8]) -> alloc::vec::Vec<u8> {
    let mut key = alloc::vec::Vec::new();
    key.extend_from_slice(BALANCE_PREFIX);
    key.extend_from_slice(account);
    key
}

fn get_balance(account: &[u8]) -> i64 {
    let bytes = storage::get(&balance_key(account)).unwrap_or_default();
    if bytes.is_empty() {
        return 0;
    }
    let mut buf = [0u8; 8];
    for (i, b) in bytes.iter().take(8).enumerate() {
        buf[i] = *b;
    }
    i64::from_le_bytes(buf)
}

fn set_balance(account: &[u8], amount: i64) {
    storage::put(&balance_key(account), &amount.to_le_bytes());
}

pub fn dispatch(method: &str) -> i32 {
    match method {
        "totalSupply" => {
            let bytes = storage::get(TOTAL_SUPPLY_KEY).unwrap_or_default();
            if bytes.is_empty() {
                return TOTAL_SUPPLY as i32;
            }
            let mut buf = [0u8; 8];
            for (i, b) in bytes.iter().take(8).enumerate() {
                buf[i] = *b;
            }
            i64::from_le_bytes(buf) as i32
        }
        "symbol" => {
            syscalls::runtime_notify("symbol", &[StackValue::ByteString(b"NEO".to_vec())]);
            0
        }
        "decimals" => DECIMALS as i32,
        "balanceOf" => {
            // In a real implementation, the account would come from method args
            syscalls::runtime_log("balanceOf called");
            0
        }
        "transfer" => {
            syscalls::runtime_notify(
                "transfer",
                &[
                    StackValue::ByteString(b"from".to_vec()),
                    StackValue::ByteString(b"to".to_vec()),
                    StackValue::Integer(0),
                ],
            );
            1
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
    fn nep17_metadata_dispatches() {
        assert_eq!(dispatch("totalSupply"), 1_000_000_000i64 as i32);
        assert_eq!(dispatch("decimals"), 8);
        assert_eq!(dispatch("transfer"), 1);
        assert_eq!(dispatch("symbol"), 0);
        assert_eq!(dispatch("unknown"), -1);
    }

    #[test]
    fn nep17_balanceOf_returns_zero() {
        // balanceOf is a stub that logs and returns 0
        assert_eq!(dispatch("balanceOf"), 0);
    }

    #[test]
    fn nep17_rejects_invalid_methods() {
        assert_eq!(dispatch(""), -1);
        assert_eq!(dispatch("mint"), -1);
        assert_eq!(dispatch("burn"), -1);
        assert_eq!(dispatch("TOTALSUPPLY"), -1);
    }
}
