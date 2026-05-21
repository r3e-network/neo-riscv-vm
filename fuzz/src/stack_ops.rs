#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_fuzz::{check_stack_values, run_with_stack};

#[path = "stack_ops_builder.rs"]
mod stack_ops_builder;

use stack_ops_builder::build_stack_ops_script;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let remaining = if data.len() > 8 { &data[8..] } else { &[] };

    let script = build_stack_ops_script(seed, remaining);

    let result = run_with_stack(&script, Vec::new());

    if let Some(result) = result {
        assert!(result.stack.len() <= 2048, "Stack overflow not caught");

        check_stack_values(&result.stack);
    }
});
