# Neo Master-N3 PolkaVM Design

## Goal

Make `neo-project/neo` on `master-n3` execute Neo contracts through a Rust runtime hosted by PolkaVM, while preserving existing Neo bytecode, NEF files, manifests, syscall names, script builders, witnesses, and contract-facing tooling.

## Constraints

- C# `neo` remains the source of truth for blockchain state, native contracts, policy, diagnostics, and event emission.
- The managed `ApplicationEngine.Create()` entry point stays stable for all existing callers.
- Neo bytecode is not translated to a new contract format in phase 1.
- Execution moves into a Rust NeoVM interpreter running inside PolkaVM.
- The migration must be incremental and testable with focused parity slices.

## Architecture

### 1. C# Compatibility Layer

`Neo.SmartContract.ApplicationEngine.Create()` already supports redirection through `IApplicationEngineProvider`. The first integration step is to install a provider that returns a `RiscvApplicationEngine` subclass. That subclass preserves the public object model expected by `Blockchain`, wallet verification helpers, diagnostics, and tests, but delegates opcode execution to a bridge instead of the managed jump table loop.

### 2. Rust Host Runtime

The Rust host runtime is a native library loaded in-process by C#. It owns the PolkaVM engine, guest lifecycle, session memory, serialization, and host-call dispatch table. The host runtime exposes a narrow C ABI for:

- creating and destroying execution sessions
- loading Neo scripts and execution metadata
- stepping or executing a session
- returning VM state, evaluation stack items, fault text, logs, and notifications
- forwarding guest host-calls back into C#

### 3. PolkaVM Guest

The PolkaVM guest is a Rust RISC-V binary that implements NeoVM semantics as a bytecode interpreter. It consumes Neo bytecode directly, maintains NeoVM stacks and contexts, and invokes host functions whenever blockchain or native-contract context is required. Phase 1 should support pure opcode execution and a minimal syscall surface before expanding toward full parity.

### 4. Host-Call Ownership

The guest never talks to chain state directly. Instead:

1. C# prepares the execution envelope.
2. Rust host launches or reuses a guest instance.
3. Guest interprets Neo bytecode.
4. Guest requests host services through PolkaVM imports.
5. Rust host converts those requests into C# callbacks.
6. C# resolves state, native contracts, and events, then returns normalized results.

This preserves `master-n3` behavior where it matters most: contract state transitions, native contract semantics, GAS accounting policy, and diagnostic reporting.

## Compatibility Surface

The following user-visible inputs remain unchanged:

- contract NEF and manifest format
- `ScriptBuilder` output
- syscall IDs and names
- transaction witness behavior
- application log and notification model
- verification and application trigger model

The following implementation detail changes:

- managed opcode loop becomes bridge-driven
- VM stack/context state is mirrored between C# and Rust
- fault reporting is synthesized from Rust interpreter state

## Delivery Strategy

Phase 1 is a vertical slice:

- provider-based engine replacement in C#
- Rust native bridge crate
- PolkaVM guest crate
- minimal Neo opcode interpreter for trivial scripts
- focused tests proving `ApplicationEngine.Create()` can redirect execution without caller changes

Later phases expand opcode coverage, syscall coverage, state snapshot integration, native contract interop, and parity testing.
