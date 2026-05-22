// Opcode test generator - generates test cases for all 256 NeoVM opcodes

use neo_riscv_abi::OpCode;

pub fn generate_opcode_tests() -> Vec<OpcodeTest> {
    let mut tests = Vec::new();

    // Generate test for every possible opcode byte.
    for opcode in 0u8..=u8::MAX {
        tests.push(OpcodeTest {
            opcode,
            name: opcode_name(opcode),
            script: vec![opcode],
        });
    }

    tests
}

#[allow(dead_code)]
pub struct OpcodeTest {
    pub opcode: u8,
    pub name: &'static str,
    pub script: Vec<u8>,
}

fn opcode_name(opcode: u8) -> &'static str {
    OpCode::try_from(opcode).map_or("UNKNOWN", OpCode::name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_256_tests() {
        let tests = generate_opcode_tests();
        assert_eq!(tests.len(), 256);
    }
}
