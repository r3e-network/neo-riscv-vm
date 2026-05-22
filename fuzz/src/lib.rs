#![no_std]

extern crate alloc;

pub mod generators;

use alloc::string::String;
use alloc::vec::Vec;
use neo_riscv_abi::{ExecutionResult, StackValue, VmState};
use neo_riscv_guest::{interpret_with_stack_and_syscalls, SyscallProvider};

#[cfg(test)]
#[path = "stack_ops_builder.rs"]
mod stack_ops_builder;

pub fn run_script(script: &[u8]) -> Option<ExecutionResult> {
    run_with_stack(script, Vec::new())
}

pub fn run_with_stack(script: &[u8], stack: Vec<StackValue>) -> Option<ExecutionResult> {
    let mut host = NoOpSyscall;
    interpret_with_stack_and_syscalls(script, stack, &mut host).ok()
}

pub fn assert_invariants(result: &ExecutionResult) {
    match result.state {
        VmState::Halt => {
            assert!(
                result.fault_message.is_none(),
                "Halt state must not have fault_message"
            );
        }
        VmState::Fault => {
            assert!(
                result.fault_message.is_some()
                    || !result.stack.is_empty()
                    || result.fee_consumed_pico > 0,
                "Fault state should have either fault_message, stack content, or gas consumed"
            );
        }
    }

    check_stack_values(&result.stack);
}

pub struct NoOpSyscall;

impl SyscallProvider for NoOpSyscall {
    fn syscall(
        &mut self,
        _api: u32,
        _ip: usize,
        _stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub fn check_stack_values(stack: &[StackValue]) {
    for value in stack {
        check_single_value(value);
    }
}

fn check_single_value(value: &StackValue) {
    match value {
        StackValue::Integer(_)
        | StackValue::Boolean(_)
        | StackValue::Null
        | StackValue::Interop(_)
        | StackValue::Iterator(_)
        | StackValue::Pointer(_) => {}
        StackValue::BigInteger(bytes) => {
            assert!(bytes.len() <= 32, "BigInteger should not exceed 32 bytes");
        }
        StackValue::ByteString(bytes) | StackValue::Buffer(bytes) => {
            assert!(
                bytes.len() <= 1024 * 1024,
                "ByteString/Buffer should not exceed 1MB"
            );
        }
        StackValue::Array(items) | StackValue::Struct(items) => {
            assert!(
                items.len() <= 1000,
                "Array/Struct should not exceed 1000 items"
            );
            for item in items {
                check_single_value(item);
            }
        }
        StackValue::Map(items) => {
            assert!(items.len() <= 1000, "Map should not exceed 1000 items");
            for (key, value) in items {
                check_single_value(key);
                check_single_value(value);
            }
        }
    }
}

pub struct SimpleRng(u64);

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_op_syscall() {
        let script = [0x11, 0x40];
        let result = run_script(&script);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.state, VmState::Halt);
    }
}
