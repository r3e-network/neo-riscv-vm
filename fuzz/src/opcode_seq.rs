#![no_main]

extern crate alloc;
extern crate libfuzzer_sys;

use alloc::vec::Vec;
use libfuzzer_sys::fuzz_target;
use neo_riscv_abi::{StackValue, VmState};

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

    let script = generate_valid_script(seed, data);

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

fn is_valid_opcode(op: u8) -> bool {
    PUSH_RANGE.contains(&op)
        || STACK_RANGE.contains(&op)
        || SLOT_RANGE.contains(&op)
        || ARITHMETIC_RANGE.contains(&op)
        || BITWISE_RANGE.contains(&op)
        || LOGICAL_RANGE.contains(&op)
        || COMPOUND_RANGE.contains(&op)
        || CONTROL_RANGE.contains(&op)
        || EXCEPTION_RANGE.contains(&op)
        || TYPE_RANGE.contains(&op)
        || SPLICE_RANGE.contains(&op)
}

fn requires_immediate(op: u8) -> Option<usize> {
    match op {
        0x00 => Some(1),
        0x01 => Some(2),
        0x02 => Some(4),
        0x03 => Some(8),
        0x04 => Some(16),
        0x05 => Some(32),
        0x0c => Some(256),
        0x0d => Some(512),
        0x0e => Some(1024),
        0x0a => Some(4),
        0x5f | 0x67 | 0x6f | 0x77 | 0x7f | 0x87 => Some(1),
        _ => None,
    }
}

fn generate_valid_script(seed: u64, _context: &[u8]) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = Vec::new();
    let target_len = 10 + (rng.next() % 50) as usize;

    while script.len() < target_len {
        let op = rng.next() as u8;
        if !is_valid_opcode(op) {
            continue;
        }

        script.push(op);

        if let Some(imm_size) = requires_immediate(op) {
            for _ in 0..imm_size {
                script.push((rng.next() % 256) as u8);
            }
        }
    }

    if script.is_empty() {
        script.push(0x40);
    } else if script[script.len() - 1] != 0x40 {
        script.push(0x40);
    }

    script
}

use core::ops::RangeInclusive;

const PUSH_RANGE: RangeInclusive<u8> = 0x00..=0x20;
const STACK_RANGE: RangeInclusive<u8> = 0x43..=0x55;
const SLOT_RANGE: RangeInclusive<u8> = 0x56..=0x87;
const ARITHMETIC_RANGE: RangeInclusive<u8> = 0x99..=0xba;
const BITWISE_RANGE: RangeInclusive<u8> = 0x90..=0x98;
const LOGICAL_RANGE: RangeInclusive<u8> = 0xab..=0xac;
const COMPOUND_RANGE: RangeInclusive<u8> = 0xbe..=0xd4;
const CONTROL_RANGE: RangeInclusive<u8> = 0x21..=0x41;
const EXCEPTION_RANGE: RangeInclusive<u8> = 0x3a..=0x3f;
const TYPE_RANGE: RangeInclusive<u8> = 0xd8..=0xdb;
const SPLICE_RANGE: RangeInclusive<u8> = 0x88..=0x8e;

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
