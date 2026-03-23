# Architecture: Syscall and Native Contract Integration

## Design Decision

**We reuse existing C# syscalls and native contracts from neo-riscv-core.**

This is an explicit architecture constraint, not an implementation detail:

- The C# Neo engine remains the only source of truth for syscall behavior.
- The C# Neo engine remains the only source of truth for native contract behavior.
- The Rust/RISC-V side only forwards requests and adapts data at the FFI boundary.
- We do **not** build a second RISC-V implementation of syscalls or native contracts.
- We prefer reusing the existing Neo system wherever possible instead of duplicating it in Rust.

The RISC-V VM does NOT reimplement syscalls or native contracts. Instead:

1. **RISC-V contracts call host via FFI**
2. **Host (C#) executes existing Neo N3 syscalls**
3. **Results return to RISC-V guest**

## Flow

```
C# ApplicationEngine (neo-riscv-core)
    ↓ Detects RISC-V contract
    ↓ Starts RISC-V VM context
Rust Host (neo-riscv-host via P/Invoke)
    ↓ Executes RISC-V bytecode
RISC-V Contract (guest code)
    ↓ Needs syscall (e.g., storage::get)
    ↓ host_call() back to Rust Host
Rust Host
    ↓ Callback to C# via P/Invoke
C# ApplicationEngine
    ↓ Executes existing Neo N3 syscall
    ↓ Returns result
Rust Host
    ↓ Returns to RISC-V guest
RISC-V Contract continues
```

## Benefits

- ✅ No duplication of syscall logic
- ✅ Automatic compatibility with Neo N3
- ✅ Reuse battle-tested C# implementations
- ✅ Minimal maintenance burden

## Devpack Role

The Rust devpack provides:

- Type-safe bindings for contracts
- Ergonomic API wrappers
- All actual execution happens in C# layer
