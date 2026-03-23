## Context

Current state: neo-riscv-vm runtime is operational and can execute RISC-V binaries. However, no toolchain exists for developers to write contracts targeting this runtime. Existing Neo contracts use C# + neo-devpack-dotnet targeting NeoVM.

Constraints:

- Must produce PolkaVM-compatible RISC-V binaries
- Must integrate with existing Neo N3 manifest and deployment flow
- Must provide familiar developer experience (similar to neo-devpack-dotnet)

## Goals / Non-Goals

**Goals:**

- Enable Rust/C contract development for RISC-V VM
- Provide standard library with Neo N3 syscalls
- Define contract packaging format (binary + manifest)
- Create deployment and invocation tools

**Non-Goals:**

- C# language support (use existing neo-devpack-dotnet for NeoVM)
- IDE integration (future work)
- Contract debugging tools (future work)
- Migration tools from NeoVM to RISC-V (future work)

## Decisions

### Decision 1: Rust as Primary Language

**Choice:** Rust with no_std for contract development
**Rationale:** Rust compiles to RISC-V, has no_std support, memory safety
**Alternative:** C/C++ → rejected, less safe; AssemblyScript → rejected, no RISC-V target

### Decision 2: Cargo-based Workflow

**Choice:** Use cargo build with custom target
**Rationale:** Standard Rust tooling, familiar to developers
**Alternative:** Custom build system → rejected, reinventing wheel

### Decision 3: Manifest Compatibility

**Choice:** Extend existing Neo manifest format with riscv-specific fields
**Rationale:** Maintains compatibility with existing tools
**Alternative:** New manifest format → rejected, breaks ecosystem

### Decision 4: Syscall ABI

**Choice:** Use PolkaVM syscall mechanism for Neo N3 interop
**Rationale:** Already implemented in neo-riscv-guest
**Alternative:** Custom FFI → rejected, duplicates work

## Risks / Trade-offs

**Risk:** Rust learning curve for C# developers
→ **Mitigation:** Provide examples, templates, migration guide

**Risk:** Binary size larger than NeoVM bytecode
→ **Mitigation:** Use release builds with LTO, document size limits

**Risk:** Debugging harder than C# contracts
→ **Accepted:** Future work, use logging for now

**Trade-off:** Rust-only initially (no C# support)
→ **Accepted:** Focus on one language, expand later
