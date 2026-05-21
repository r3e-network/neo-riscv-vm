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

    assert!(source.contains("normalize_stack_item_type_tag"));
    assert!(source.contains("default_value_for_type_tag"));
    assert!(!source.contains("const NEO_TAG_"));
    assert!(!source.contains("fn normalize_type_tag"));
}

#[test]
fn collections_use_shared_new_array_default_helper() {
    let source =
        fs::read_to_string(rt_src_path("collections.rs")).expect("collections source is readable");

    assert!(source.contains("new_array_default_value_for_type_tag"));
    assert!(!source.contains("fn default_for_type"));
}
