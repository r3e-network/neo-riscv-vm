//! Audit H14: `panic = "abort"` in the workspace profiles made every FFI
//! `catch_unwind` boundary in `crates/neo-riscv-host/src/ffi.rs` dead code —
//! the process would abort at the first panic instead of returning a FAULT
//! receipt to Neo. This test fails whenever the host is built with
//! `-C panic=abort` again; the guest crate is exempt (its target has no
//! unwinder, and `regenerate-guest-blob.sh` passes `-C panic=abort` itself).

#[test]
fn host_profiles_do_not_abort_on_panic() {
    assert!(
        !cfg!(panic = "abort"),
        "neo-riscv-host must build with unwinding: panic=abort turns the ten FFI catch_unwind arms in ffi.rs into dead code, so a Rust panic aborts the process instead of returning a FAULT receipt (audit H14)"
    );
}
