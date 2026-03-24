# Current Status

**Date:** 2026-03-24  
**Status:** Workspace production-ready

## Executive Summary

The current committed implementation is a validated three-repo workspace integration:

- `neo-riscv-vm` provides the Rust runtime, NeoVM-on-RISC-V guest, adapter plugin, compatibility tests, fuzz harnesses, and orchestration scripts.
- `neo-riscv-core` no longer contains its own `Neo.SmartContract.RiscV` bridge layer. That code now lives in `neo-riscv-vm/compat/Neo.Riscv.Adapter`.
- `neo-riscv-node` is validated against the packaged adapter bundle, including CLI smoke coverage.
- Existing C# syscall and native-contract implementations remain the only source of truth.

## Architecture Reality

### What is true now

- NeoVM compatibility is provided by the Rust guest interpreter running on top of PolkaVM.
- The adapter plugin registers `ApplicationEngine.Provider` and resolves the native host library.
- Core is generic and now expects provider injection instead of auto-resolving an in-core RISC-V engine.
- Native contract execution and syscall behavior are still executed and charged in C#.

### What is no longer true

- The workspace is no longer a literal “zero changes to core and node” deployment.
- `neo-riscv-core` now carries a focused refactor that externalizes the RISC-V bridge to the adapter.
- `neo-riscv-node` carries targeted CLI/plugin testability fixes.

### What remains unchanged

- Existing user contract semantics are preserved.
- Syscall/native-contract behavior is still defined by the Neo C# engine.
- The adapter continues to support both NeoVM compatibility execution and native RISC-V contract execution.

## Validation Evidence

Fresh committed-state verification passed with:

### VM-local verification

- `cargo test --workspace --all-targets`
  - `206` guest tests
  - `93` host tests
  - `12` devpack tests
- `cargo clippy -p neo-riscv-abi -p neo-riscv-guest -p neo-riscv-host --tests -- -D warnings`
- `cargo test --manifest-path fuzz/Cargo.toml --lib`
- `cargo build --manifest-path fuzz/Cargo.toml --bins`

### Compatibility and integration verification

- `./scripts/verify-all.sh`
  - full NeoVM JSON corpus runner (`161` copied corpus files)
  - adapter tests
- `./tests/e2e/run-all.sh`
- `./scripts/test-ffi-resolution.sh`
- `dotnet build src/Neo/Neo.csproj` in `neo-riscv-core`
- targeted core extraction/provider slice: `82/82` tests passed
- `./scripts/cross-repo-test.sh`
  - core matrix: `1179` tests passed
  - node matrix: `477` tests passed
  - `neo-cli` smoke passed

## Known Coupling / Residual Risk

- The current `neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj` references the sibling `neo-riscv-vm/compat/Neo.Riscv.Adapter` project directly.
- That makes the current integration workspace-valid, but not yet a standalone-upstream-clean core repo shape.

## Canonical Commands

Full integrated validation:

```bash
./scripts/cross-repo-test.sh
```

Local VM validation:

```bash
cargo test --workspace --all-targets
./scripts/verify-all.sh
./tests/e2e/run-all.sh
```

Standalone fuzz package:

```bash
cargo test --manifest-path fuzz/Cargo.toml --lib
cargo build --manifest-path fuzz/Cargo.toml --bins
```
