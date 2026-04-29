#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::StackValue;

#[path = "stack_ops_builder.rs"]
mod stack_ops_builder;

use stack_ops_builder::build_stack_ops_script;

struct NoOpSyscall;

impl neo_riscv_guest::SyscallProvider for NoOpSyscall {
    fn syscall(
        &mut self,
        _api: u32,
        _ip: usize,
        _stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        Ok(())
    }
}

fn run_with_stack(script: &[u8], stack: Vec<StackValue>) -> Option<neo_riscv_abi::ExecutionResult> {
    let mut host = NoOpSyscall;
    neo_riscv_guest::interpret_with_stack_and_syscalls(script, stack, &mut host).ok()
}

fn check_result_value(value: &StackValue) {
    match value {
        StackValue::Integer(_)
        | StackValue::Boolean(_)
        | StackValue::Null
        | StackValue::Pointer(_)
        | StackValue::Interop(_)
        | StackValue::Iterator(_) => {}
        StackValue::BigInteger(bytes) => {
            assert!(bytes.len() <= 32, "BigInteger exceeds max size");
        }
        StackValue::ByteString(bytes) | StackValue::Buffer(bytes) => {
            assert!(bytes.len() <= 1024 * 1024, "ByteString/Buffer exceeds max size");
        }
        StackValue::Array(items) | StackValue::Struct(items) => {
            assert!(items.len() <= 1000, "Array/Struct too large");
            for item in items {
                check_result_value(item);
            }
        }
        StackValue::Map(items) => {
            assert!(items.len() <= 1000, "Map too large");
            for (k, v) in items {
                check_result_value(k);
                check_result_value(v);
            }
        }
    }
}

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

        for val in &result.stack {
            check_result_value(val);
        }
    }
});
