# Validation Complete

**Date:** 2026-03-26
**Status:** Cross-repo validation passing (production-hardened)

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
| VM Rust/devpack workspace tests | ✅ `376` tests passed |
| Full JSON corpus compatibility | ✅ `161` corpus files validated |
| Core matrix | ✅ `1,169` tests passed (89 + 92 + 988) |
| Node matrix | ✅ `477` tests passed |
| VM E2E + FFI smoke | ✅ passed |
| `neo-cli` smoke | ✅ passed |
| **Total** | **2,022 tests passing cross-repo** |

Canonical validation command:

```bash
./scripts/cross-repo-test.sh
```

## Production Hardening Validation

As of 2026-03-26, the implementation has undergone 8 review cycles with 34 fixes applied:

- Interpreter correctness fixes (XDROP, ENDTRY, CALL/RET, JMPEQ)
- Security hardening (codec limits, OOM guards)
- C# adapter and bridge fixes
- Reliability improvements and expanded test coverage

All fixes are covered by the test counts above.

## Important Clarification

The original project goal was a literal zero-change drop-in replacement.

The **current committed implementation is not that exact shape**:

- `neo-riscv-core` now externalizes the RISC-V bridge into the adapter project
- `neo-riscv-node` includes targeted integration/testability fixes
- user contracts remain compatible, but the workspace itself is now a deliberate multi-repo integration

See [Current Status](./docs/CURRENT_STATUS.md) for the exact architecture and caveats.
