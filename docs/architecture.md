> **Note:** The canonical, comprehensive architecture document is [ARCHITECTURE.md](./ARCHITECTURE.md). This file is kept for historical reference.

# Architecture

## Goal

Replace direct NeoVM execution in Neo N3 core with a Rust-backed RISC-V runtime built on PolkaVM, while preserving the existing contract model, script format, NEF format, manifests, and tooling.

## Layering

### C# Neo Core

The C# side remains authoritative for:

- chain state
- storage snapshots
- witness verification semantics
- notifications and logs
- native contract behavior
- fee policy and protocol settings

The RISC-V bridge does not reimplement blockchain state in Rust.

### Rust Host

The Rust host:

- exports the native ABI Neo loads through `NativeLibrary.Load`
- loads a PolkaVM guest blob
- marshals stack values between C# and Rust
- forwards syscalls back to the C# bridge
- returns final VM state, result stack, and fee consumption

### PolkaVM Guest

The PolkaVM guest:

- receives script bytes, initial stack, and entry offset
- executes Neo script opcodes implemented in Rust guest logic
- serializes syscall stack state back to the host
- receives updated stack state from the host after each syscall

One important implementation detail is that the guest replaces the runtime stack wholesale after a successful syscall result import instead of clearing and extending the existing vector in place. On the PolkaVM RISC-V target this avoids corrupting subsequent large dynamic-call arguments after host callbacks, while preserving the visible NeoVM stack semantics.

## ABI Shape

The ABI is centered on:

- `StackValue`
- `ExecutionResult`
- serialized stack/result payloads carried with the custom fast codec (formerly `postcard`)

Supported stack classes include:

- integers / big integers
- byte strings
- booleans
- arrays
- structs
- maps
- iterator handles
- generic interop handles
- null

## Execution Context Rules

The bridge preserves Neo execution context semantics by explicitly switching engine context for:

- native contract direct calls
- `System.Contract.CallNative`
- nested contract calls executed through the Rust bridge
- verification flows with stacked invocation / verification contexts

## Fee Model

Fee accounting is coordinated between the Rust host and the C# bridge:

- The PolkaVM guest reports every executed Neo opcode back to the Rust host through `host_on_instruction`.
- The Rust host applies Neo opcode prices to its `RuntimeContext`, updates `gas_left`, and accumulates `fee_consumed_pico`.
- Syscall fixed prices, native contract CPU/storage fees, and explicit runtime burns remain charged in C#.
- The C# bridge uses the Rust-supplied `gas_left` snapshot during syscall callbacks and applies the Rust-reported `fee_consumed_pico` after execution completes.

This keeps `GasLeft` and final `FeeConsumed` aligned with Neo’s existing execution semantics while preserving C# as the source of truth for interop/native policy charges.

## Current Packaging Constraint

The host currently embeds a checked-in `guest.polkavm` blob.

That is functionally verified, but artifact regeneration is still an explicit step documented in the repo rather than a fully transparent build-time pipeline.

The verified regeneration path is:

- build the guest with nightly Cargo for `riscv32emac-unknown-none-polkavm`
- link the resulting ELF with `polkatool link --strip`
- replace the checked-in `crates/neo-riscv-guest-module/guest.polkavm`
