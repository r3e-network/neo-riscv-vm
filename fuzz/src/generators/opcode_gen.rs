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

pub fn is_valid_opcode(op: u8) -> bool {
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

pub fn requires_immediate(op: u8) -> Option<usize> {
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

pub fn generate_valid_script(seed: u64) -> alloc::vec::Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = alloc::vec::Vec::new();
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

pub const PUSH_RANGE: RangeInclusive<u8> = 0x00..=0x20;
pub const STACK_RANGE: RangeInclusive<u8> = 0x43..=0x55;
pub const SLOT_RANGE: RangeInclusive<u8> = 0x56..=0x87;
pub const ARITHMETIC_RANGE: RangeInclusive<u8> = 0x99..=0xba;
pub const BITWISE_RANGE: RangeInclusive<u8> = 0x90..=0x98;
pub const LOGICAL_RANGE: RangeInclusive<u8> = 0xab..=0xac;
pub const COMPOUND_RANGE: RangeInclusive<u8> = 0xbe..=0xd4;
pub const CONTROL_RANGE: RangeInclusive<u8> = 0x21..=0x41;
pub const EXCEPTION_RANGE: RangeInclusive<u8> = 0x3a..=0x3f;
pub const TYPE_RANGE: RangeInclusive<u8> = 0xd8..=0xdb;
pub const SPLICE_RANGE: RangeInclusive<u8> = 0x88..=0x8e;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_ranges() {
        assert!(PUSH_RANGE.contains(&0x11));
        assert!(PUSH_RANGE.contains(&0x00));
        assert!(PUSH_RANGE.contains(&0x20));
        assert!(!PUSH_RANGE.contains(&0x21));
    }

    #[test]
    fn test_is_valid_opcode() {
        assert!(is_valid_opcode(0x11));
        assert!(is_valid_opcode(0x9e));
        assert!(is_valid_opcode(0x40));
        assert!(!is_valid_opcode(0xff));
        assert!(!is_valid_opcode(0x42));
    }

    #[test]
    fn test_requires_immediate() {
        assert_eq!(requires_immediate(0x00), Some(1));
        assert_eq!(requires_immediate(0x01), Some(2));
        assert_eq!(requires_immediate(0x11), None);
        assert_eq!(requires_immediate(0x40), None);
    }

    #[test]
    fn test_generate_valid_script() {
        let script = generate_valid_script(12345);
        assert!(!script.is_empty());
        assert_eq!(script[script.len() - 1], 0x40);
    }
}
