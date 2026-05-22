use std::{fs, path::Path};

#[test]
fn contract_runtime_is_part_of_guest_crate_not_a_separate_workspace_crate() {
    let guest_manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = guest_manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("guest crate lives under crates/");

    let workspace_manifest =
        fs::read_to_string(workspace_root.join("Cargo.toml")).expect("workspace Cargo.toml");

    assert!(
        !workspace_manifest.contains("\"crates/neo-riscv-rt\""),
        "contract runtime must live under neo-riscv-guest::contract_rt"
    );
    assert!(
        !workspace_root.join("crates/neo-riscv-rt").exists(),
        "retired neo-riscv-rt crate directory must not remain in the workspace"
    );
}
