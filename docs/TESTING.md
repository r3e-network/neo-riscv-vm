# Testing Guide

**Version:** 1.1  
**Last Updated:** 2026-03-24

## Overview

The current testing strategy has two layers:

1. **VM-local verification** inside `neo-riscv-vm`
2. **Cross-repo matrix verification** across `neo-riscv-vm`, `neo-riscv-core`, and `neo-riscv-node`

The canonical full validation command is:

```bash
./scripts/cross-repo-test.sh
```

## Current Suite Boundaries

| Scope | Command | Coverage |
|------|---------|----------|
| VM workspace | `cargo test --workspace --all-targets` | guest, host, devpack |
| Compat corpus | `./scripts/verify-all.sh` | full copied NeoVM JSON corpus + adapter tests |
| VM smoke | `./tests/e2e/run-all.sh` / `./scripts/test-ffi-resolution.sh` | E2E + FFI |
| Cross-repo | `./scripts/cross-repo-test.sh` | VM + core + node + CLI smoke |

## Latest Verified Counts

### VM workspace

- guest tests: `206`
- host tests: `93`
- devpack tests: `12`

### Compatibility / adapter

- copied NeoVM JSON corpus files: `161`
- adapter tests: `7`

### Core matrix

- `Neo.Extensions.Tests`: `89`
- `Neo.Json.UnitTests`: `92`
- `Neo.UnitTests`: `990`
- core total: `1171`

### Node matrix

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
- node total: `477`

## Recommended Commands

### Fast local VM check

```bash
cargo test --workspace --all-targets
```

### Full VM compatibility check

```bash
./scripts/verify-all.sh
./tests/e2e/run-all.sh
./scripts/test-ffi-resolution.sh
```

### Full integrated matrix

```bash
./scripts/cross-repo-test.sh
```

## Important Notes

- `scripts/verify-all.sh` runs the full copied NeoVM corpus in `NEO_RISCV_VM_JSON_MODE=full`.
- `scripts/cross-repo-test.sh` is the authoritative integration proof because it packages the adapter, runs VM checks, runs core tests, runs node tests, and finishes with the `neo-cli` smoke flow.
- The current core integration no longer requires a direct sibling adapter `ProjectReference` to compile.
- RISC-V bridge coverage on the core side still requires the adapter assembly to be available at runtime through the staged plugin bundle or an explicit adapter path.
