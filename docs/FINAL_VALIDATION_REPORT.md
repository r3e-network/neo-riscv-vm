# Final Validation Report

**Date:** 2026-03-24  
**Status:** Cross-repo validation passing

## Executive Summary

The committed implementation is validated as a plugin-first, adapter-owned RISC-V execution stack for Neo N3.

Key facts:

- NeoVM compatibility runs through the Rust guest interpreter on PolkaVM.
- Existing C# syscalls and native contracts remain the source of truth.
- The RISC-V bridge/provider code now lives in `neo-riscv-vm/compat/Neo.Riscv.Adapter`, not in `neo-riscv-core`.
- Full integrated validation passed across `neo-riscv-vm`, `neo-riscv-core`, and `neo-riscv-node`.

## Verification Evidence

### VM-local verification

`cargo test --workspace --all-targets` passed with:

- `206` guest tests
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
- `161` copied NeoVM JSON corpus files available under `compat/Neo.VM.Riscv.Tests/Corpus/Tests`
- compatibility runner tests passing
- adapter tests passing

### Core verification

`./scripts/cross-repo-test.sh` passed the core matrix:

- `Neo.Extensions.Tests`: `89`
- `Neo.Json.UnitTests`: `92`
- `Neo.UnitTests`: `998`

Core total: `1179`

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

## Operational Interpretation

The current implementation should be described as:

- **contract-compatible**
- **adapter-first**
- **workspace production-ready**

It should **not** be described as a literal zero-change upstream drop-in anymore, because:

- `neo-riscv-core` now contains a targeted refactor that externalizes the RISC-V bridge to the adapter project
- `neo-riscv-node` now contains targeted integration/testability fixes

## Residual Risk / Known Caveats

- `neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj` currently references the sibling adapter project in `neo-riscv-vm`, so the current shape is workspace-coupled.

## Canonical Command

```bash
./scripts/cross-repo-test.sh
```

That command is the best current proof that the committed VM, core, node, adapter bundle, and CLI smoke path all work together.
