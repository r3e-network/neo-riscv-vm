# Final Validation Report

**Date:** 2026-03-26
**Status:** Cross-repo validation passing (production-hardened)

## Executive Summary

The committed implementation is validated as a plugin-first, adapter-owned RISC-V execution stack for Neo N3.

Key facts:

- NeoVM compatibility runs through the Rust guest interpreter on PolkaVM.
- Existing C# syscalls and native contracts remain the source of truth.
- The RISC-V bridge/provider code now lives in `neo-riscv-vm/dotnet/Neo.Riscv.Adapter`, not in `neo-riscv-core`.
- Full integrated validation passed across `neo-riscv-vm`, `neo-riscv-core`, and `neo-riscv-node`.
- Core test compilation no longer depends on a direct sibling adapter project reference.
- Plugin startup no longer aborts the process when a config watcher cannot be allocated.

## Verification Evidence

### VM-local verification

`cargo test --workspace --all-targets` passed with:

- `271` guest tests
- `93` host tests
- `12` devpack tests

Additional VM-local checks that passed:

- `cargo clippy -p neo-riscv-abi -p neo-riscv-guest -p neo-riscv-host --tests -- -D warnings`
- `cargo test --manifest-path fuzz/Cargo.toml --lib`
- `cargo build --manifest-path fuzz/Cargo.toml --bins`
- `./tests/e2e/run-all.sh`
- `./scripts/test-ffi-resolution.sh`

### Compatibility verification

`./scripts/verify-all.sh` passed with:

- full JSON corpus mode enabled
- `161` copied NeoVM JSON corpus files available under `dotnet/Neo.VM.Riscv.Tests/Corpus/Tests`
- compatibility runner tests passing
- adapter tests passing (`10`)

### Core verification

`./scripts/cross-repo-test.sh` passed the core matrix:

- `Neo.Extensions.Tests`: `89`
- `Neo.Json.UnitTests`: `92`
- `Neo.UnitTests`: `988`

Core total: `1,169`

Additional focused verification:

- `dotnet build src/Neo/Neo.csproj` passed with `0` warnings and `0` errors
- affected provider/adapter slice passed `82/82`

### Node verification

`./scripts/cross-repo-test.sh` passed the node matrix:

- `Neo.CLI.Tests`: `29`
- `Neo.ConsoleService.Tests`: `13`
- `Neo.Cryptography.MPTTrie.Tests`: `55`
- `Neo.Network.RPC.Tests`: `95`
- `Neo.Plugins.ApplicationLogs.Tests`: `15`
- `Neo.Plugins.DBFTPlugin.Tests`: `34`
- `Neo.Plugins.OracleService.Tests`: `3`
- `Neo.Plugins.RestServer.Tests`: `11`
- `Neo.Plugins.RpcServer.Tests`: `164`
- `Neo.Plugins.SQLiteWallet.Tests`: `41`
- `Neo.Plugins.SignClient.Tests`: `5`
- `Neo.Plugins.StateService.Tests`: `8`
- `Neo.Plugins.Storage.Tests`: `4`

Node total: `477`

Additional smoke verification:

- `neo-cli` publish + packaged plugin smoke passed

### Cross-repo totals

| Scope | Tests |
|-------|-------|
| VM workspace | 376 |
| Core matrix | 1,169 |
| Node matrix | 477 |
| **Total** | **2,022** |

### Production Hardening Fixes (2026-03-26)

Following 8 review cycles, 34 fixes have been applied across the codebase:

- **Interpreter correctness (4 areas):** XDROP boundary handling, ENDTRY control flow, CALL/RET frame management, and JMPEQ offset semantics corrected to match C# NeoVM behavior.
- **Security hardening:** Codec size limits to prevent unbounded deserialization, OOM guards for guest memory allocation, bounds checking on stack/slot operations.
- **C# adapter fixes:** Adapter bridge marshalling corrections, provider registration edge cases, plugin lifecycle handling.
- **Reliability and quality:** Improved error propagation, panic-safety in FFI boundaries, expanded test coverage for edge cases discovered during review.

All fixes are validated by the test counts above.

## Operational Interpretation

The current implementation should be described as:

- **contract-compatible**
- **adapter-first**
- **workspace production-ready**

It should **not** be described as a literal zero-change upstream drop-in anymore, because:

- `neo-riscv-core` now contains a targeted refactor that externalizes the RISC-V bridge to the adapter project
- `neo-riscv-node` now contains targeted integration/testability fixes

## Residual Risk / Known Caveats

- `neo-riscv-core` test compilation no longer requires the sibling adapter project directly.
- The integrated RISC-V bridge tests still rely on the adapter assembly being staged at runtime, so the current validation story remains intentionally cross-repo rather than fully standalone.

## Canonical Command

```bash
./scripts/cross-repo-test.sh
```

That command is the best current proof that the committed VM, core, node, adapter bundle, and CLI smoke path all work together.
