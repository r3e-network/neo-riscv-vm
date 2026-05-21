use std::fs;
use std::path::PathBuf;

fn rt_src_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file_name)
}

#[test]
fn conversion_uses_shared_stack_item_type_helpers() {
    let source =
        fs::read_to_string(rt_src_path("conversion.rs")).expect("conversion source is readable");

    assert!(source.contains("semantics::runtime::conversion"));
    assert!(!source.contains("const NEO_TAG_"));
    assert!(!source.contains("fn normalize_type_tag"));
    assert!(!source.contains("fn convert_to_integer"));
    assert!(!source.contains("default_value_for_type_tag"));
}

#[test]
fn collections_use_shared_collection_semantics() {
    let source =
        fs::read_to_string(rt_src_path("collections.rs")).expect("collections source is readable");

    assert!(source.contains("semantics::runtime::collections"));
    assert!(!source.contains("fn default_for_type"));
    assert!(!source.contains("new_array_default_value_for_type_tag"));
    assert!(!source.contains("Vec::with_capacity"));
}

#[test]
fn strings_use_shared_byte_sequence_helpers() {
    let source = fs::read_to_string(rt_src_path("strings.rs")).expect("strings source is readable");

    assert!(source.contains("concat_byte_sequences"));
    assert!(source.contains("slice_byte_sequence"));
    assert!(source.contains("byte_sequence_len"));
    assert!(source.contains("byte_sequence_bytes"));
    assert!(!source.contains("StackValue::ByteString(mut"));
    assert!(!source.contains("index + count"));
    assert!(!source.contains("di + count"));
}

#[test]
fn opcode_modules_use_shared_abi_semantics() {
    let arithmetic =
        fs::read_to_string(rt_src_path("arithmetic.rs")).expect("arithmetic source is readable");
    let comparison =
        fs::read_to_string(rt_src_path("comparison.rs")).expect("comparison source is readable");
    let conversion =
        fs::read_to_string(rt_src_path("conversion.rs")).expect("conversion source is readable");
    let collections =
        fs::read_to_string(rt_src_path("collections.rs")).expect("collections source is readable");

    assert!(arithmetic.contains("semantics::runtime::arithmetic"));
    assert!(!arithmetic.contains("pop_integer"));
    assert!(!arithmetic.contains("fn mod_pow_i64"));
    assert!(!arithmetic.contains("fn isqrt"));

    assert!(comparison.contains("semantics::runtime::comparison"));
    assert!(!comparison.contains("pop_integer"));
    assert!(conversion.contains("semantics::runtime::conversion"));
    assert!(collections.contains("semantics::runtime::collections"));
    assert!(!collections.contains("new_array_default_value_for_type_tag"));
}
