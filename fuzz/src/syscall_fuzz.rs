#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::StackValue;

struct FuzzingSyscall {
    seed: u64,
}

impl neo_riscv_guest::SyscallProvider for FuzzingSyscall {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let mut rng = SimpleRng::new(self.seed.wrapping_add(api as u64));

        match api {
            0xf6fc79b2 => {
                stack.push(StackValue::ByteString(b"NEO".to_vec()));
            }
            0xa0387de9 => {
                stack.push(StackValue::Integer(0));
            }
            0xced88814 => {
                stack.push(StackValue::Integer(1000000));
            }
            _ => {
                let result_count = (rng.next() % 3) as usize;
                for i in 0..result_count {
                    let val_seed = rng.next().wrapping_add(i as u64);
                    let choice = (val_seed % 6) as u8;
                    match choice {
                        0 => stack.push(StackValue::Integer(val_seed as i64)),
                        1 => stack.push(StackValue::Boolean(val_seed % 2 == 0)),
                        2 => stack.push(StackValue::Null),
                        3 => {
                            let len = 1 + (val_seed % 8) as usize;
                            let mut bytes = Vec::with_capacity(len);
                            for j in 0..len {
                                bytes.push((val_seed.wrapping_add(j as u64) % 256) as u8);
                            }
                            stack.push(StackValue::ByteString(bytes));
                        }
                        4 => stack.push(StackValue::Array(Vec::new())),
                        _ => stack.push(StackValue::Integer(val_seed as i64)),
                    }
                }
            }
        }

        self.seed = rng.next();
        Ok(())
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let mut rng = SimpleRng::new(self.seed.wrapping_add(token as u64));

        let result_count = (rng.next() % 2) as usize;
        for _i in 0..result_count {
            stack.push(StackValue::Integer(rng.next() as i64));
        }

        self.seed = rng.next();
        Ok(())
    }
}

fn run_with_fuzzing_syscall(script: &[u8], seed: u64) -> Option<neo_riscv_abi::ExecutionResult> {
    let mut host = FuzzingSyscall { seed };
    neo_riscv_guest::interpret_with_stack_and_syscalls(script, Vec::new(), &mut host).ok()
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let syscall_api = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let remaining = if data.len() > 12 { &data[12..] } else { &[] };

    let script = build_syscall_script(syscall_api, seed, remaining);

    let result = run_with_fuzzing_syscall(&script, seed);

    if let Some(result) = result {
        for val in &result.stack {
            check_result_value(val);
        }
    }
});

fn build_syscall_script(api: u32, seed: u64, context: &[u8]) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = Vec::new();

    let push_ops = [0x11, 0x12, 0x13, 0x14, 0x15, 0x0b, 0x09, 0x08];

    let prep_count = if context.is_empty() {
        3
    } else {
        core::cmp::min(context.len(), 5)
    };

    for i in 0..prep_count {
        let push_op = push_ops[(rng.next() as usize + i) % push_ops.len()];
        script.push(push_op);
    }

    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());

    script.push(0x40);

    script
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
        StackValue::ByteString(bytes) => {
            assert!(bytes.len() <= 1024 * 1024, "ByteString exceeds max size");
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

#[cfg(test)]
mod tests {
    use super::*;
    use neo_riscv_abi::interop_hash;

    #[test]
    fn test_syscall_fuzzing() {
        let api = interop_hash("System.Runtime.Platform");
        let script = build_syscall_script(api, 12345, &[]);
        let result = run_with_fuzzing_syscall(&script, 12345);
        assert!(result.is_some());
    }
}
