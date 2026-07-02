use std::fs;
use std::path::PathBuf;

/// Compile-time proof the boundary codec modules are re-exported: this crate has
/// no such modules of its own, so a successful import means they resolve through
/// the `pub use neo_vm_rs::{...}` re-export in lib.rs.
#[allow(unused_imports)]
use neo_riscv_abi::{callback_codec, fast_codec, result_codec};

fn abi_lib_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");
    fs::read_to_string(path).expect("abi lib.rs should be readable")
}

/// Guard: the boundary codecs must come from neo-vm-rs, not be re-implemented in
/// this crate. They are re-exported as modules directly in lib.rs (previously via
/// one-line stub files). Keeping this structural check prevents a future edit
/// from reintroducing a second, drifting codec implementation here.
#[test]
fn codecs_are_reexported_from_shared_vm_crate() {
    let source = abi_lib_source();
    assert!(
        source.contains("pub use neo_vm_rs::{callback_codec, fast_codec, result_codec};"),
        "the three boundary codec modules must be re-exported directly from neo-vm-rs"
    );
}
