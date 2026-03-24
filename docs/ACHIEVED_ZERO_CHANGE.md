# Historical Zero-Change Target

This document is retained as a historical note.

The original target was a literal zero-change replacement of NeoVM with no core or node changes. The committed 2026-03-24 workspace no longer matches that exact statement.

## Target vs Current Reality

| Item | Historical Target | Current Committed State |
|------|-------------------|-------------------------|
| User contract compatibility | Zero migration | ✅ Preserved |
| Syscall/native contract source of truth | C# engine | ✅ Preserved |
| In-core RISC-V bridge | Allowed | ❌ Removed from core, now externalized to adapter |
| Core repo unchanged | Yes | ❌ Targeted integration refactor committed |
| Node repo unchanged | Yes | ❌ Targeted CLI/plugin fixes committed |
| Plugin deployment | Adapter bundle | ✅ Preserved |

## What still matters from the original goal

- Existing contracts still execute through the compatibility layer.
- Native contract logic is still not reimplemented in Rust.
- The adapter can still be packaged as a plugin bundle.

## What changed

- `neo-riscv-core` now treats the RISC-V engine as an external provider supplied by `Neo.Riscv.Adapter`.
- `neo-riscv-node` now includes supporting changes for redirected input, plugin staging, and CLI smoke reproducibility.
- The accurate description is now “workspace-scoped adapter integration”, not “zero changes everywhere”.

See [Current Status](./CURRENT_STATUS.md) for the current architecture.
