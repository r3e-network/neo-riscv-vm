use std::fs;
use std::path::PathBuf;

fn rt_src_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file_name)
}

fn read_rt_src(file_name: &str) -> String {
    fs::read_to_string(rt_src_path(file_name)).expect("runtime source is readable")
}

#[test]
fn opcode_adapters_are_consolidated_into_one_shared_vm_bridge() {
    for retired_module in [
        "arithmetic.rs",
        "comparison.rs",
        "conversion.rs",
        "collections.rs",
        "opcode_adapters.rs",
        "strings.rs",
    ] {
        assert!(
            !rt_src_path(retired_module).exists(),
            "{retired_module} should not duplicate shared VM runtime semantics"
        );
    }

    let source = read_rt_src("lib.rs");

    assert!(source.contains("VmContext"));
    assert!(!source.contains("pub stack:"));
    assert!(!source.contains("pub locals:"));
    assert!(!source.contains("pub args:"));
    assert!(!source.contains("pub static_fields:"));

    for retired_method in [
        "pub fn add(",
        "pub fn sub(",
        "pub fn mul(",
        "pub fn equal(",
        "pub fn cat(",
        "pub fn substr(",
        "pub fn left(",
        "pub fn right(",
        "pub fn memcpy(",
    ] {
        assert!(
            !source.contains(retired_method),
            "{retired_method} should be provided by neo-vm-rs runtime APIs"
        );
    }

    assert!(!source.contains("pop_integer"));
    assert!(!source.contains("fn mod_pow_i64"));
    assert!(!source.contains("fn isqrt"));
    assert!(!source.contains("const NEO_TAG_"));
    assert!(!source.contains("fn normalize_type_tag"));
    assert!(!source.contains("fn convert_to_integer"));
    assert!(!source.contains("fn default_for_type"));
    assert!(!source.contains("default_value_for_type_tag"));
    assert!(!source.contains("new_array_default_value_for_type_tag"));
    assert!(!source.contains("Vec::with_capacity"));
}

#[test]
fn riscv_runtime_does_not_define_private_byte_opcode_helpers() {
    let source = read_rt_src("lib.rs");

    assert!(!source.contains("concat_byte_sequences"));
    assert!(!source.contains("slice_byte_sequence"));
    assert!(!source.contains("byte_sequence_len"));
    assert!(!source.contains("byte_sequence_bytes"));
    assert!(!source.contains("StackValue::ByteString(mut"));
    assert!(!source.contains("index + count"));
    assert!(!source.contains("di + count"));
}
