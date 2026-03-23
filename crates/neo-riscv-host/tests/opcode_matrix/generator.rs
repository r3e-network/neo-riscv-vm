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
        0x02 => "PUSHINT32",
        0x08 => "PUSHT",
        0x09 => "PUSHF",
        0x0b => "PUSHNULL",
        0x0c => "PUSHDATA1",
        0x24 => "JMPIF",
        0x26 => "JMPIFNOT",
        0x40 => "RET",
        0x45 => "DROP",
        0x4a => "DUP",
        0x50 => "SWAP",
        0x51 => "ROT",
        0x53 => "REVERSE3",
        0x88 => "NEWBUFFER",
        0x8b => "CAT",
        0x9e => "ADD",
        0x9f => "SUB",
        0xa0 => "MUL",
        0xa1 => "DIV",
        0xa2 => "MOD",
        0xd9 => "ISTYPE",
        0xdb => "CONVERT",
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
