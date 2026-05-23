#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::OpCode;
use neo_riscv_fuzz::{check_stack_values, run_with_stack, SimpleRng};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let remaining = if data.len() > 8 { &data[8..] } else { &[] };

    let script = build_type_conv_script(seed, remaining);

    let result = run_with_stack(&script, Vec::new());

    if let Some(result) = result {
        check_stack_values(&result.stack);
    }
});

fn build_type_conv_script(seed: u64, context: &[u8]) -> Vec<u8> {
    let _rng = SimpleRng::new(seed);
    let mut script = Vec::new();

    let ops = [
        OpCode::ISTYPE.byte(),
        OpCode::CONVERT.byte(),
        OpCode::SIZE.byte(),
        OpCode::HASKEY.byte(),
        OpCode::KEYS.byte(),
        OpCode::VALUES.byte(),
    ];

    for (i, &byte) in context.iter().enumerate() {
        if ops.contains(&byte) {
            script.push(byte);
            if i + 1 < context.len() {
                script.push(context[i + 1]);
            }
        }
        if script.len() >= 20 {
            break;
        }
    }

    if script.is_empty() {
        script.push(OpCode::PUSH1.byte());
        script.push(OpCode::ISTYPE.byte());
        script.push(OpCode::PUSH0.byte());
        script.push(OpCode::RET.byte());
    } else {
        script.push(OpCode::RET.byte());
    }

    script
}
