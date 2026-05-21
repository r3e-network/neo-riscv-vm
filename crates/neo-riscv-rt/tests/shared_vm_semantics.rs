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
    ] {
        assert!(
            !rt_src_path(retired_module).exists(),
            "{retired_module} should not duplicate shared VM runtime semantics"
        );
    }

    let source = read_rt_src("opcode_adapters.rs");

    assert!(source.contains("semantics::runtime::{"));
    assert!(source.contains("arithmetic as vm_arithmetic"));
    assert!(source.contains("comparison as vm_comparison"));
    assert!(source.contains("conversion as vm_conversion"));
    assert!(source.contains("collections as vm_collections"));

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
fn strings_use_shared_byte_sequence_helpers() {
    let source = read_rt_src("strings.rs");

    assert!(source.contains("concat_byte_sequences"));
    assert!(source.contains("slice_byte_sequence"));
    assert!(source.contains("byte_sequence_len"));
    assert!(source.contains("byte_sequence_bytes"));
    assert!(!source.contains("StackValue::ByteString(mut"));
    assert!(!source.contains("index + count"));
    assert!(!source.contains("di + count"));
}
