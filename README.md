# Neo RISC-V VM

[![Validation](https://img.shields.io/badge/validation-cross--repo%20passing-brightgreen)](./docs/FINAL_VALIDATION_REPORT.md)
[![Status](https://img.shields.io/badge/status-workspace%20production%20ready-brightgreen)](./docs/CURRENT_STATUS.md)
[![Syscalls](https://img.shields.io/badge/syscalls-C%23%20source%20of%20truth-blue)](./docs/architecture-syscalls.md)

Production-ready RISC-V execution stack for Neo N3, hardened through 8 review cycles with 34 fixes applied across correctness, security, reliability, and code quality.

The current committed implementation is a plugin-first, cross-repo integration:

- `neo-riscv-vm` owns the Rust runtime, guest interpreter, adapter plugin, docs, and validation scripts.
- `neo-riscv-core` now stays generic and no longer carries an in-core `Neo.SmartContract.RiscV` bridge implementation.
- `neo-riscv-node` is validated with the packaged adapter bundle and CLI smoke coverage.
- Existing C# syscall and native-contract logic remains the source of truth.
- core test compilation no longer depends on a direct sibling adapter project reference.

This preserves contract compatibility while avoiding a second Rust/RISC-V implementation of syscalls or native contracts.

## Current Status

- NeoVM bytecode runs inside the PolkaVM guest interpreter.
- Native RISC-V contracts and legacy NeoVM contracts share the same adapter bridge surface.
- `ApplicationEngine.Provider` must now be supplied explicitly by the adapter plugin or tests; core no longer auto-resolves an in-core RISC-V provider.
- The integrated three-repo workspace is validated end to end.
- This is not a literal “zero changes to core and node” state anymore. The current design is a workspace-scoped externalized adapter architecture.

See [Current Status](./docs/CURRENT_STATUS.md) for the exact architecture and caveats.

## Validation Snapshot

Latest committed verification passed with:

| Scope | Evidence |
|------|----------|
| VM workspace tests | `cargo test --workspace --all-targets` passed (`376` Rust/devpack tests) |
| JSON compatibility | full corpus runner passed over `161` copied NeoVM JSON files |
| Adapter tests | `NEO_RISCV_HOST_LIB=... dotnet test compat/Neo.Riscv.Adapter.Tests/...` passed (`10` tests) |
| Core matrix | `1,169` tests passed across `Neo.Extensions.Tests` (89), `Neo.Json.UnitTests` (92), and `Neo.UnitTests` (988) |
| Node matrix | `477` tests passed across `13` node/plugin test projects |
| Smoke coverage | VM E2E, FFI resolution, and `neo-cli` smoke all passed |
| **Total** | **2,022 tests passing cross-repo** |

Canonical full validation command:

```bash
./scripts/cross-repo-test.sh
```

Detailed evidence is recorded in [Final Validation Report](./docs/FINAL_VALIDATION_REPORT.md).

## Architecture

```text
Neo core / node
  -> adapter plugin registers ApplicationEngine.Provider
  -> NativeRiscvVmBridge P/Invokes libneo_riscv_host.so
  -> PolkaVM executes guest.polkavm for NeoVM bytecode
  -> host callbacks route syscalls and native contract calls back to C#
```

Key architectural rules:

- C# remains the syscall and native-contract source of truth.
- The adapter package owns the RISC-V bridge/provider implementation.
- Core is now generic: no in-core `Neo.SmartContract.RiscV` subtree remains.
- Repeated plugin/test startup no longer hard-fails when a filesystem watcher cannot be allocated.
- The current validation/deployment model is a packaged plugin bundle, not a zero-diff upstream drop-in.

## Quick Start

Build and package the adapter bundle:

```bash
cargo build -p neo-riscv-host --release
./scripts/package-adapter-plugin.sh
```

Run the full integrated validation matrix:

```bash
./scripts/cross-repo-test.sh
```

Run local VM validation only:

```bash
cargo test --workspace --all-targets
./scripts/verify-all.sh
./tests/e2e/run-all.sh
```

Run standalone fuzz package checks:

```bash
cargo test --manifest-path fuzz/Cargo.toml --lib
cargo build --manifest-path fuzz/Cargo.toml --bins
```

## Repository Layout

```text
neo-riscv-vm/
├── crates/
│   ├── neo-riscv-abi/
│   ├── neo-riscv-guest/
│   ├── neo-riscv-guest-module/
│   ├── neo-riscv-host/
│   └── neo-riscv-devpack/
├── compat/
│   ├── Neo.Riscv.Adapter/
│   ├── Neo.Riscv.Adapter.Tests/
│   └── Neo.VM.Riscv.Tests/
├── fuzz/
├── scripts/
├── tests/
└── docs/
```

## Related Repositories

This RISC-V VM is part of a multi-repo architecture. All repos are available at [github.com/r3e-network](https://github.com/r3e-network):

| Repository | Description | Language |
|------------|-------------|----------|
| [neo-riscv-vm](https://github.com/r3e-network/neo-riscv-vm) | RISC-V VM execution engine (this repo) | Rust |
| [neo-riscv-node](https://github.com/r3e-network/neo-riscv-node) | Neo node with RISC-VM support | C# |
| [neo-riscv-core](https://github.com/r3e-network/neo-riscv-core) | Core Neo library with RISC-VM support | C# |
| [neo-riscv-devpack](https://github.com/r3e-network/neo-riscv-devpack) | Rust smart contract development kit | Rust |

### Using as Dependencies

**C# Projects** - Reference via project dependencies or git submodules:
```xml
<!-- In Directory.Build.props -->
<NeoSiblingCoreProject>$(MSBuildThisFileDirectory)neo-riscv-core\src\Neo\Neo.csproj</NeoSiblingCoreProject>
```

**Rust Projects** - Use git dependencies:
```toml
[dependencies]
neo-riscv-devpack = { git = "https://github.com/r3e-network/neo-riscv-devpack" }
```

## Documentation

- [Current Status](./docs/CURRENT_STATUS.md)
- [Final Validation Report](./docs/FINAL_VALIDATION_REPORT.md)
- [Testing Guide](./docs/TESTING.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [NEP-RISC-V-VM](./docs/NEP-RISC-V-VM.md)
- [API Reference](./docs/API_REFERENCE.md)
- [Syscall Architecture](./docs/architecture-syscalls.md)
- [Native Contracts](./docs/native-contracts.md)

Historical zero-change design notes are retained here for context, but they no longer describe the exact committed workspace state:

- [Historical Zero-Change Target](./docs/ACHIEVED_ZERO_CHANGE.md)
- [Historical Zero-Change Architecture](./docs/ZERO_CHANGE_ARCHITECTURE.md)
