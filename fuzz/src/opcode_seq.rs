#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::VmState;
use neo_riscv_fuzz::{generators::generate_valid_script, run_with_stack};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);

    let script = generate_valid_script(seed);

    if script.is_empty() {
        return;
    }

    let result = run_with_stack(&script, Vec::new());

    if let Some(result) = result {
        match result.state {
            VmState::Halt | VmState::Fault => {}
        }
    }
});
