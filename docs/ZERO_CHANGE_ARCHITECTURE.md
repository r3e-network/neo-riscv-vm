# Historical Zero-Change Architecture

This file documents the original zero-change ambition.

It no longer describes the exact committed workspace shape as of 2026-03-24.

## Current Architecture Instead

```text
neo-riscv-core
  -> generic ApplicationEngine.Provider hook
  -> no in-core Neo.SmartContract.RiscV bridge subtree

neo-riscv-vm/compat/Neo.Riscv.Adapter
  -> plugin entry point
  -> provider resolver
  -> ApplicationEngine implementation
  -> native bridge / FFI

neo-riscv-node
  -> validated against packaged adapter bundle
  -> CLI smoke and plugin staging fixes committed
```

## Historical Claim vs Current State

- Historical claim: “no core or node code changes required”
- Current state: targeted core/node changes are part of the validated workspace

## Still-Valid Design Intent

- Contracts still see NeoVM-compatible behavior through the guest interpreter.
- C# remains authoritative for syscalls, fees, and native contracts.
- The adapter remains the integration seam between Neo and the RISC-V runtime.

For the current production-ready description, use [Current Status](./CURRENT_STATUS.md) and [Architecture](./ARCHITECTURE.md).
