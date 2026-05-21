use std::fs;
use std::path::PathBuf;

fn abi_src_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file_name)
}

#[test]
fn callback_codec_is_reexported_from_shared_vm_crate() {
    let source = fs::read_to_string(abi_src_path("callback_codec.rs"))
        .expect("callback codec source should be readable");

    assert!(source.contains("pub use neo_vm_rs::callback_codec::*;"));
}

#[test]
fn result_codec_is_reexported_from_shared_vm_crate() {
    let source = fs::read_to_string(abi_src_path("result_codec.rs"))
        .expect("result codec source should be readable");

    assert!(source.contains("pub use neo_vm_rs::result_codec::*;"));
}
