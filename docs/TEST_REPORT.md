# Test Report

**Date:** 2026-03-24

## Summary

The current implementation passes both local VM verification and the full three-repo integration matrix.

### Local VM verification

- `cargo test --workspace --all-targets`
- `cargo clippy -p neo-riscv-abi -p neo-riscv-guest -p neo-riscv-host --tests -- -D warnings`
- `cargo test --manifest-path fuzz/Cargo.toml --lib`
- `cargo build --manifest-path fuzz/Cargo.toml --bins`

### Integrated verification

- `./scripts/verify-all.sh`
- `./tests/e2e/run-all.sh`
- `./scripts/test-ffi-resolution.sh`
- `./scripts/cross-repo-test.sh`

## What this validates

- NeoVM-on-RISC-V guest execution
- native RISC-V contract dispatch path
- adapter/provider bootstrap
- C# syscall/native-contract source-of-truth routing
- core integration
- node integration
- packaged plugin CLI workflow

## Current interpretation

The validation outcome supports a **workspace production-ready** claim.

It does **not** support the older absolute “zero changes to core and node” claim, because the validated workspace now includes:

- targeted `neo-riscv-core` refactoring to externalize the bridge
- targeted `neo-riscv-node` integration/testability fixes

See [Current Status](./CURRENT_STATUS.md) for the precise architecture description.
