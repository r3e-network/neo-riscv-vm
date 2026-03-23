// Opcode test generator - generates test cases for all 256 NeoVM opcodes

pub fn generate_opcode_tests() -> Vec<OpcodeTest> {
    let mut tests = Vec::new();

    // Generate test for each opcode 0x00-0xFF
    for opcode in 0u8..=255 {
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
    match opcode {
        0x00 => "PUSHINT8",
        0x01 => "PUSHINT16",
        // Placeholder - full mapping to be added
        _ => "UNKNOWN",
    }
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
