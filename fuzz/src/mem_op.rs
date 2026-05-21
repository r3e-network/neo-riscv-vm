#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_fuzz::{check_stack_values, run_with_stack, SimpleRng};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let remaining = if data.len() > 8 { &data[8..] } else { &[] };

    let script = build_mem_op_script(seed, remaining);

    let result = run_with_stack(&script, Vec::new());

    if let Some(result) = result {
        check_stack_values(&result.stack);
    }
});

fn build_mem_op_script(seed: u64, context: &[u8]) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = Vec::new();

    script.push(0x88);

    let size_byte = context
        .first()
        .copied()
        .unwrap_or_else(|| (rng.next() % 256) as u8);
    script.push(size_byte.saturating_add(1));

    let mem_ops = [0x8b, 0x8c, 0x8d, 0x8e, 0x89, 0xca];

    for (i, &byte) in context.iter().enumerate() {
        if mem_ops.contains(&byte) {
            script.push(byte);

            if byte == 0x89 && i + 3 < context.len() {
                script.push(context[i + 1]);
                script.push(context[i + 2]);
                script.push(context[i + 3]);
            } else if (byte == 0x8c || byte == 0x8d || byte == 0x8e) && i + 2 < context.len() {
                script.push(context[i + 1]);
                script.push(context[i + 2]);
            }
        }

        if script.len() >= 30 {
            break;
        }
    }

    if script.len() < 3 {
        script.push(0x11);
        script.push(0x8b);
        script.push(0x40);
    } else {
        script.push(0x40);
    }

    script
}
