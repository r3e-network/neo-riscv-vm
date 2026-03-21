# RISC-V VM Current State

## Verified State

As of 2026-03-18, the Rust workspace and the matching C# Neo `master-n3` worktree are green together:

- Rust guest/host tests pass
- C# `Neo.UnitTests` pass with the Rust host library enabled

## What Is Implemented

- C# `ApplicationEngine.Create()` resolves the RISC-V provider path by default when the native library is available
- Neo execution is routed through `RiscvApplicationEngine`
- The Rust host executes a PolkaVM guest module
- The guest/host/C# bridge supports:
  - runtime syscalls
  - storage syscalls
  - iterator handles
  - native contract direct calls
  - `System.Contract.CallNative`
  - verification scripts
  - custom witness scripts
  - generic interop handle round-tripping
  - opcode-fee accounting

## What Is Not Finished

The guest-module regeneration path is documented and reproducible, but not yet transparently generated as part of every host build without caveats.

That is the main remaining packaging/build concern after the runtime/test work completed.
