#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::StackValue;

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
        for val in &result.stack {
            check_mem_value(val);
        }
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

fn check_mem_value(value: &StackValue) {
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
        StackValue::ByteString(bytes) => {
            assert!(bytes.len() <= 1024 * 1024, "ByteString exceeds max size");
        }
        StackValue::Array(items) | StackValue::Struct(items) => {
            assert!(items.len() <= 1000, "Array/Struct too large");
            for item in items {
                check_mem_value(item);
            }
        }
        StackValue::Map(items) => {
            assert!(items.len() <= 1000, "Map too large");
            for (k, v) in items {
                check_mem_value(k);
                check_mem_value(v);
            }
        }
    }
}

pub struct SimpleRng(u64);

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }
}
