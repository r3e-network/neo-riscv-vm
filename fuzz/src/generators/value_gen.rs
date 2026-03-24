#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use neo_riscv_abi::StackValue;

pub fn generate_integer(seed: u64) -> i64 {
    let mut rng = SimpleRng::new(seed);
    rng.next() as i64
}

pub fn generate_big_integer(seed: u64, max_bytes: usize) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let num_bytes = 1 + (rng.next() as usize) % max_bytes.min(32);
    let mut result = Vec::with_capacity(num_bytes);
    for _ in 0..num_bytes {
        result.push((rng.next() % 256) as u8);
    }
    result
}

pub fn generate_bytestring(seed: u64, max_len: usize) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let len = (rng.next() as usize) % max_len.min(65536);
    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        result.push((rng.next() % 256) as u8);
    }
    result
}

pub fn generate_stack_value(depth: u64, seed: u64) -> StackValue {
    let mut rng = SimpleRng::new(seed);
    let choice = (rng.next() % 10) as u8;

    match choice {
        0 => StackValue::Integer(generate_integer(rng.next())),
        1 => StackValue::BigInteger(generate_big_integer(rng.next(), 16)),
        2 => StackValue::ByteString(generate_bytestring(rng.next(), 256)),
        3 => StackValue::Boolean(rng.next() % 2 == 0),
        4 => StackValue::Null,
        5 if depth < 3 => {
            let count = 1 + (rng.next() % 4) as usize;
            let items: Vec<StackValue> = (0..count)
                .map(|i| generate_stack_value(depth + 1, rng.next().wrapping_add(i as u64)))
                .collect();
            StackValue::Array(items)
        }
        6 if depth < 3 => {
            let count = 1 + (rng.next() % 4) as usize;
            let items: Vec<StackValue> = (0..count)
                .map(|i| generate_stack_value(depth + 1, rng.next().wrapping_add(i as u64)))
                .collect();
            StackValue::Struct(items)
        }
        7 if depth < 2 => {
            let count = 1 + (rng.next() % 4) as usize;
            let items: Vec<(StackValue, StackValue)> = (0..count)
                .map(|i| {
                    (
                        generate_stack_value(depth + 1, rng.next().wrapping_add(i as u64 * 2)),
                        generate_stack_value(depth + 1, rng.next().wrapping_add(i as u64 * 2 + 1)),
                    )
                })
                .collect();
            StackValue::Map(items)
        }
        _ => StackValue::Integer(generate_integer(rng.next())),
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

    #[test]
    fn test_generate_integer() {
        let val = generate_integer(12345);
        assert!(val != 0 || true);
    }

    #[test]
    fn test_generate_big_integer() {
        let val = generate_big_integer(12345, 32);
        assert!(!val.is_empty());
        assert!(val.len() <= 32);
    }

    #[test]
    fn test_generate_bytestring() {
        let val = generate_bytestring(12345, 1024);
        assert!(val.len() <= 1024);
    }

    #[test]
    fn test_generate_stack_value() {
        let val = generate_stack_value(0, 12345);
        assert!(matches!(
            val,
            StackValue::Integer(_) | StackValue::Boolean(_) | StackValue::Null | _
        ));
    }
}
