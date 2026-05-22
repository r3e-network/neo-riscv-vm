use alloc::vec::Vec;
use crate::SimpleRng;
use neo_riscv_abi::OpCode;

pub fn is_valid_opcode(op: u8) -> bool {
    OpCode::try_from(op).is_ok()
}

pub fn requires_immediate(op: u8) -> Option<usize> {
    let opcode = OpCode::try_from(op).ok()?;
    match opcode.operand_size() {
        0 => None,
        size => Some(size),
    }
}

pub fn generate_valid_script(seed: u64) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    let mut script = Vec::new();
    let target_len = 10 + (rng.next() % 50) as usize;

    while script.len() < target_len {
        let opcode = match OpCode::try_from(rng.next() as u8) {
            Ok(opcode) => opcode,
            Err(_) => continue,
        };

        script.push(opcode.byte());
        append_operands(&mut script, opcode, &mut rng);
    }

    if script.last().copied() != Some(OpCode::RET.byte()) {
        script.push(OpCode::RET.byte());
    }

    script
}

fn append_operands(script: &mut Vec<u8>, opcode: OpCode, rng: &mut SimpleRng) {
    match opcode {
        OpCode::PUSHDATA1 => append_pushdata1(script, rng),
        OpCode::PUSHDATA2 => append_pushdata2(script, rng),
        OpCode::PUSHDATA4 => append_pushdata4(script, rng),
        _ => append_fixed_operands(script, opcode.operand_size(), rng),
    }
}

fn append_pushdata1(script: &mut Vec<u8>, rng: &mut SimpleRng) {
    let len = (rng.next() % 32) as u8;
    script.push(len);
    append_random_bytes(script, len as usize, rng);
}

fn append_pushdata2(script: &mut Vec<u8>, rng: &mut SimpleRng) {
    let len = (rng.next() % 64) as u16;
    script.extend_from_slice(&len.to_le_bytes());
    append_random_bytes(script, len as usize, rng);
}

fn append_pushdata4(script: &mut Vec<u8>, rng: &mut SimpleRng) {
    let len = (rng.next() % 128) as u32;
    script.extend_from_slice(&len.to_le_bytes());
    append_random_bytes(script, len as usize, rng);
}

fn append_fixed_operands(script: &mut Vec<u8>, len: usize, rng: &mut SimpleRng) {
    append_random_bytes(script, len, rng);
}

fn append_random_bytes(script: &mut Vec<u8>, len: usize, rng: &mut SimpleRng) {
    for _ in 0..len {
        script.push((rng.next() % 256) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_ranges() {
        assert!(is_valid_opcode(OpCode::PUSH1.byte()));
        assert!(is_valid_opcode(OpCode::PUSHINT8.byte()));
        assert!(is_valid_opcode(OpCode::PUSH16.byte()));
        assert!(is_valid_opcode(OpCode::NOP.byte()));
    }

    #[test]
    fn test_is_valid_opcode() {
        assert!(is_valid_opcode(OpCode::PUSH1.byte()));
        assert!(is_valid_opcode(OpCode::ADD.byte()));
        assert!(is_valid_opcode(OpCode::RET.byte()));
        assert!(!is_valid_opcode(u8::MAX));
        assert!(!is_valid_opcode(OpCode::SYSCALL.byte() + 1));
    }

    #[test]
    fn test_requires_immediate() {
        assert_eq!(requires_immediate(OpCode::PUSHINT8.byte()), Some(1));
        assert_eq!(requires_immediate(OpCode::PUSHINT16.byte()), Some(2));
        assert_eq!(requires_immediate(OpCode::PUSH1.byte()), None);
        assert_eq!(requires_immediate(OpCode::RET.byte()), None);
    }

    #[test]
    fn test_generate_valid_script() {
        let script = generate_valid_script(12345);
        assert!(!script.is_empty());
        assert_eq!(script[script.len() - 1], OpCode::RET.byte());
    }
}
