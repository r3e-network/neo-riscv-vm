#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::VmState;
use neo_riscv_fuzz::{run_with_stack, SimpleRng};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let remaining = if data.len() > 8 { &data[8..] } else { &[] };

    let script = build_exception_script(seed, remaining);

    let result = run_with_stack(&script, Vec::new());

    if let Some(result) = result {
        match result.state {
            VmState::Halt => {
                assert!(
                    result.fault_message.is_none(),
                    "Halt state should not have fault_message"
                );
            }
            VmState::Fault => {}
        }
    }
});

fn build_exception_script(seed: u64, context: &[u8]) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = Vec::new();

    script.push(0x11);
    script.push(0x11);

    let remaining_len = if context.is_empty() {
        10
    } else {
        core::cmp::min(context.len(), 20)
    };

    for i in 0..remaining_len {
        let byte = context.get(i).copied().unwrap_or_else(|| rng.next() as u8);
        let choice = rng.next() % 10;

        match choice {
            0 => {
                script.push(0x3b);
                script.push((rng.next() % 5) as u8);
            }
            1 => {
                script.push(0x3d);
            }
            2 => {
                script.push(0x3a);
            }
            3 => {
                script.push(0xf1);
            }
            4 => {
                script.push(0x3f);
            }
            _ => {
                if byte != 0x40 && (byte == 0x11 || byte == 0x12 || byte == 0x4a) {
                    script.push(byte);
                }
            }
        }

        if script.len() >= 30 {
            break;
        }
    }

    script.push(0x40);

    script
}
