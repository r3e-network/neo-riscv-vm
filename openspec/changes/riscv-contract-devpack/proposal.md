## Why

Neo N3 contracts currently compile to NeoVM bytecode. With the RISC-V VM runtime now operational, developers need a complete toolchain to write, compile, and deploy contracts targeting RISC-V instead of legacy NeoVM.

## What Changes

- Create RISC-V contract compiler (Rust/C to RISC-V binary)
- Build devpack with standard library and Neo N3 syscall/native-contract bindings that forward to the existing C# Neo implementation
- Define contract standards (manifest format, entry points, ABI)
- Implement deployment tools (binary + manifest packaging)
- Add contract invocation utilities

## Explicit Reuse Constraint

- Syscalls remain implemented in the existing C# Neo engine.
- Native contracts remain implemented in the existing C# Neo engine.
- The RISC-V/devpack side provides bindings and forwarding only.
- This change does **not** introduce a second RISC-V implementation of syscalls or native contracts.

## Capabilities

### New Capabilities

- `riscv-compiler`: Compile Rust/C contracts to RISC-V binary format
- `contract-devpack`: Standard library with Neo N3 syscalls and types
- `contract-standards`: Manifest format, ABI, entry point conventions
- `deployment-tools`: Package and deploy RISC-V contracts to Neo N3
- `invocation-utilities`: Call deployed RISC-V contracts

### Modified Capabilities

<!-- No existing capabilities are changing -->

## Impact

- **New crates**: compiler, devpack, deployment tools
- **New standards**: RISC-V contract manifest schema
- **Integration**: Works with existing neo-riscv-vm runtime
- **Developer workflow**: New compile → package → deploy pipeline
