# Current Status

**Date:** 2026-04-09
**Status:** Workspace integration validated (evidence-based)

## Executive Summary

The current committed implementation is a validated three-repo workspace integration:

- `neo-riscv-vm` provides the Rust runtime, NeoVM-on-RISC-V guest, adapter plugin, compatibility tests, fuzz harnesses, and orchestration scripts.
- `neo-riscv-core` no longer contains its own `Neo.SmartContract.RiscV` bridge layer. That code now lives in `neo-riscv-vm/compat/Neo.Riscv.Adapter`.
- `neo-riscv-node` is validated against the packaged adapter bundle, including CLI smoke coverage.
- Existing C# syscall and native-contract implementations remain the only source of truth.
- `neo-riscv-core` test compilation no longer requires a direct sibling `ProjectReference` to the adapter project.
- repeated plugin initialization no longer hard-fails when a `FileSystemWatcher` cannot be created.

## Architecture Reality

### What is true now

- NeoVM compatibility is provided by the Rust guest interpreter running on top of PolkaVM.
- The adapter plugin registers `ApplicationEngine.Provider` and resolves the native host library.
- Adapter library lookup now resolves from straightforward published/plugin filesystem locations rather than depending on `Neo.Plugins.Plugin` static initialization.
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
  - `271` guest tests
  - `93` host tests
  - `12` devpack tests
- `cargo clippy -p neo-riscv-abi -p neo-riscv-guest -p neo-riscv-host --tests -- -D warnings`
- `cargo test --manifest-path fuzz/Cargo.toml --lib`
- `cargo build --manifest-path fuzz/Cargo.toml --bins`

### Compatibility and integration verification

- `./scripts/verify-all.sh`
  - full NeoVM JSON corpus runner (`161` copied corpus files)
  - adapter tests (`10`)
- `TIME_PER_TARGET=2 RUNS_PER_TARGET=5 FUZZ_SEED=123 NEO_RUN_FUZZ=1 ./scripts/verify-all.sh`
  - bounded instrumented libFuzzer run for `opcode_seq`, `type_convert`, `stack_ops`, `exception_handling`, `syscall_fuzz`, `whole_system_parity`, and `mem_op`
- `./tests/e2e/run-all.sh`
- `./scripts/test-ffi-resolution.sh`
- `dotnet build src/Neo/Neo.csproj` in `neo-riscv-core`
- targeted core extraction/provider slice: `82/82` tests passed
- `./scripts/cross-repo-test.sh`
  - core matrix: `1,169` tests passed (89 + 92 + 988)
  - node matrix: `477` tests passed
  - `neo-cli` smoke passed
- `dotnet test neo-devpack-dotnet.sln --configuration Release --no-build -m:1`
  - `Neo.Compiler.CSharp.UnitTests`: `1107/1107`
  - `Neo.SmartContract.Framework.UnitTests`: `239/239`
  - `Neo.SmartContract.Testing.UnitTests`: `49/49`
  - `Neo.SmartContract.Template.UnitTests`: `37/37`
  - `Neo.SmartContract.Analyzer.UnitTests`: `132/132`
  - two `*TestContracts` assemblies are build artifacts and report no discoverable tests
- `cargo +nightly fuzz run whole_system_parity -- -max_total_time=30 -seed=123`
  - grew the checked workspace corpus from `2` seed files to `27` corpus entries
- **Total cross-repo: 2,022 tests passing**

## Hardening Snapshot (2026-03-26)

Following 8 review cycles, 34 fixes have been applied across the codebase:

- **Interpreter correctness:** Fixed XDROP boundary handling, ENDTRY control flow, CALL/RET frame management, and JMPEQ offset semantics to match C# NeoVM behavior exactly.
- **Security hardening:** Added codec size limits to prevent unbounded deserialization, OOM guards for guest memory allocation, and bounds checking on stack/slot operations.
- **C# adapter fixes:** Corrected adapter bridge marshalling, provider registration edge cases, and plugin lifecycle handling.
- **Reliability and quality:** Improved error propagation, panic-safety in FFI boundaries, and test coverage for edge cases discovered during review.

## Known Coupling / Residual Risk

- Integrated adapter coverage still depends on a staged plugin bundle or explicit adapter assembly path when running the core-side RISC-V bridge tests.
- The compile-time sibling-project coupling has been removed, but the runtime-integrated validation path remains intentionally cross-repo.

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
cd fuzz
cargo +nightly fuzz build opcode_seq
```

Bounded fuzz validation:

```bash
scripts/run-bounded-fuzz.sh
```

Set `TIME_PER_TARGET`, `RUNS_PER_TARGET`, or `FUZZ_SEED` to shrink or stabilize the run, e.g. `TIME_PER_TARGET=10 RUNS_PER_TARGET=5 FUZZ_SEED=123 scripts/run-bounded-fuzz.sh`. The script uses `cargo +nightly fuzz run` for instrumented libFuzzer execution rather than plain release binaries. The same script can also be invoked after `verify-all` by enabling `NEO_RUN_FUZZ=1 scripts/verify-all.sh`, so CI or developers can opt into the bounded fuzz gate without changing the default flow.
