# Validation Complete

**Date:** 2026-03-24  
**Status:** Cross-repo validation passing

## Summary

The committed workspace is validated and clean across:

- `/home/neo/git/neo-riscv-vm`
- `/home/neo/git/neo-riscv-core`
- `/home/neo/git/neo-riscv-node`

The current state is:

- NeoVM compatibility running on the RISC-V guest/runtime stack
- adapter-owned bridge/provider implementation
- C# syscalls and native contracts preserved as source of truth
- full core/node integration verified with the packaged plugin

## Current Validation Evidence

| Scope | Result |
|------|--------|
| VM Rust/devpack workspace tests | ✅ `311` tests passed |
| Full JSON corpus compatibility | ✅ `161` corpus files validated |
| Core matrix | ✅ `1179` tests passed |
| Node matrix | ✅ `477` tests passed |
| VM E2E + FFI smoke | ✅ passed |
| `neo-cli` smoke | ✅ passed |

Canonical validation command:

```bash
./scripts/cross-repo-test.sh
```

## Important Clarification

The original project goal was a literal zero-change drop-in replacement.

The **current committed implementation is not that exact shape**:

- `neo-riscv-core` now externalizes the RISC-V bridge into the adapter project
- `neo-riscv-node` includes targeted integration/testability fixes
- user contracts remain compatible, but the workspace itself is now a deliberate multi-repo integration

See [Current Status](./docs/CURRENT_STATUS.md) for the exact architecture and caveats.
