use std::{fs, path::Path};

#[test]
fn host_opcode_pricing_uses_shared_opcode_enum() {
    let source = read_source("src/pricing.rs");

    assert!(
        source.contains("OpCode::try_from"),
        "opcode pricing should classify canonical opcodes through neo-vm-rs::OpCode"
    );
    assert!(
        !source.contains("0x22..=0x33")
            && !source.contains("0x0f | 0x10..=0x21")
            && !source.contains("0xcd | 0xcf..=0xd1 | 0xdb"),
        "opcode pricing must not maintain hard-coded opcode byte groups"
    );
}

#[test]
fn host_bridge_uses_shared_stack_value_codec_tags() {
    let source = read_source("src/bridge.rs");

    assert!(
        source.contains("STACK_VALUE_CODEC_TAG_BYTESTRING"),
        "host bridge should parse raw stack payloads with shared codec tag constants"
    );
    assert!(
        !source.contains("0x03 | 0x02 | 0x0C") && !source.contains("0x01 | 0x0B | 0x08 | 0x09"),
        "host bridge must not duplicate stack-value codec tag bytes"
    );
}

#[test]
fn fuzz_opcode_generation_uses_shared_opcode_enum() {
    let source = read_workspace_source("fuzz/src/generators/opcode_gen.rs");

    assert!(
        source.contains("OpCode::try_from") && source.contains("opcode.operand_size()"),
        "fuzz opcode generation should use shared opcode metadata"
    );
    assert!(
        !source.contains("0x00 => Some(1)")
            && !source.contains("0x00..=0x20")
            && !source.contains("0x5f | 0x67 | 0x6f | 0x77 | 0x7f | 0x87"),
        "fuzz opcode generation must not duplicate opcode byte tables"
    );
}

#[test]
fn opcode_matrix_names_use_shared_opcode_enum() {
    let source = read_source("tests/opcode_matrix/generator.rs");

    assert!(
        source.contains("OpCode::try_from") && source.contains("OpCode::name"),
        "opcode matrix generation should name opcodes through shared metadata"
    );
    assert!(
        !source.contains("0x00 => \"PUSHINT8\"") && !source.contains("0x9e => \"ADD\""),
        "opcode matrix generation must not duplicate opcode-name byte tables"
    );
}

#[test]
fn fuzz_targets_use_shared_opcode_enum() {
    for relative_path in ["fuzz/src/mem_op.rs", "fuzz/src/exception_handling.rs"] {
        let source = read_workspace_source(relative_path);

        assert!(
            source.contains("OpCode::"),
            "{relative_path} should build bytecode through shared opcode metadata"
        );
        assert!(
            !source.contains("0x8c || byte == 0x8d || byte == 0x8e")
                && !source.contains("byte != 0x40")
                && !source.contains("byte == 0x11 || byte == 0x12 || byte == 0x4a"),
            "{relative_path} must not duplicate opcode bytes in fuzz control logic"
        );
    }
}

#[test]
fn examples_and_benches_build_bytecode_from_shared_opcode_enum() {
    for relative_path in [
        "crates/neo-riscv-host/examples/profile_hotspot.rs",
        "crates/neo-riscv-host/benches/benchmarks/arithmetic.rs",
        "crates/neo-riscv-host/benches/benchmarks/control_flow.rs",
        "crates/neo-riscv-host/benches/benchmarks/overhead.rs",
        "crates/neo-riscv-host/benches/benchmarks/stack_ops.rs",
        "crates/neo-riscv-host/benches/propagate_update_bench.rs",
    ] {
        let source = read_workspace_source(relative_path);

        assert!(
            source.contains("OpCode::") && source.contains(".byte()"),
            "{relative_path} should build bytecode through neo-vm-rs opcode metadata"
        );
        for duplicate in [
            "const PUSHINT8: u8 = 0x00",
            "const PUSH1: u8 = 0x11",
            "const NOP: u8 = 0x21",
            "const RET: u8 = 0x40",
            "const DUP: u8 = 0x4a",
            "const ADD: u8 = 0x9e",
            "const NEWARRAY0: u8 = 0xc2",
            "const NEWARRAY: u8 = 0xc3",
            "const APPEND: u8 = 0xcf",
            "const SETITEM: u8 = 0xd0",
            "0x08, 0x24, 0x02, 0x11",
            "0x11, 0x12, 0x9e",
            "0x11, 0x4a, 0x50, 0x45",
            "script.push(0x40)",
            "script.push(0x45)",
        ] {
            assert!(
                !source.contains(duplicate),
                "{relative_path} must not duplicate NeoVM opcode byte constants: {duplicate}"
            );
        }
    }
}

fn read_source(relative_path: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path))
        .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"))
}

fn read_workspace_source(relative_path: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative_path),
    )
    .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"))
}
