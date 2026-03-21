# neo-riscv-vm

Rust-side RISC-V / PolkaVM runtime for Neo N3 `master-n3`, paired with a C# Neo core worktree that executes contracts through the Rust host instead of the legacy NeoVM execution loop.

## What This Repository Contains

- `crates/neo-riscv-abi`
  Shared stack/result ABI used by the Rust guest, Rust host, and the C# bridge.
- `crates/neo-riscv-guest`
  Neo script execution logic and ABI-facing runtime behavior.
- `crates/neo-riscv-guest-module`
  The PolkaVM guest program that exports the entrypoints the host invokes.
- `crates/neo-riscv-host`
  Native Rust host runtime exposed as `libneo_riscv_host.so` for the C# Neo core bridge.

## Current Execution Model

- Neo C# core uses the `RiscvApplicationEngine` path by default when the native host library is available.
- The Rust host executes a compiled PolkaVM guest module (`guest.polkavm`) and bridges syscalls back into C#.
- Chain state, policy logic, storage, witnesses, logs, notifications, and native contract semantics remain source-of-truth in C#.
- Existing scripts, manifests, NEF handling, and contract tooling remain unchanged at the Neo surface.

## C# Integration

The matching C# worktree is:

- `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter`

The important C# integration points are:

- `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/ApplicationEngine.cs`
- `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvApplicationEngine.cs`
- `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/NativeRiscvVmBridge.cs`

## Build

Build the Rust host library:

```bash
cargo build -p neo-riscv-host
```

This produces:

```text
target/debug/libneo_riscv_host.so
```

## Test

Rust:

```bash
cargo test -p neo-riscv-guest -p neo-riscv-host
```

C# Neo core worktree:

```bash
NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so \
dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj
```

## Guest Module Regeneration

The host currently includes the checked-in guest blob:

- `crates/neo-riscv-guest-module/guest.polkavm`

To rebuild it from the guest module source:

1. Build the guest ELF on the PolkaVM target with nightly Cargo:

```bash
cargo +nightly build \
  --manifest-path crates/neo-riscv-guest-module/Cargo.toml \
  --release \
  --target "$(polkatool get-target-json-path -b 32)" \
  -Zbuild-std=core,alloc \
  --target-dir target
```

2. Convert the ELF into a `.polkavm` blob with `polkatool 0.32.x`:

```bash
polkatool link \
  --strip \
  -o crates/neo-riscv-guest-module/guest.polkavm \
  target/riscv32emac-unknown-none-polkavm/release/neo-riscv-guest-module
```

Notes:

- The guest must be built for the PolkaVM `riscv32emac-unknown-none-polkavm` target. The older `riscv32imac-unknown-none-elf` flow does not emit the PolkaVM export metadata the host needs.
- `--strip` is compatible with the working PolkaVM target flow and preserves the exported guest entrypoints.
- The generated blob must export the guest entrypoints the host expects: `alloc`, `execute`, `get_result_ptr`, `get_result_len`.
- The checked-in convenience script `scripts/regenerate-guest-blob.sh` uses the verified command sequence above.

## Unattended Verification

Run the full Rust plus Neo `master-n3` verification surface with:

```bash
scripts/verify-all.sh
```

## Runtime Requirement

Neo C# core now expects the RISC-V host library when no explicit `ApplicationEngine.Provider` override is injected.

The resolver checks:

- `NEO_RISCV_HOST_LIB`
- `AppContext.BaseDirectory/libneo_riscv_host.so`
- `Environment.CurrentDirectory/libneo_riscv_host.so`

On Linux, the most direct setup is still:

```bash
export NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so
```

## Status

Verified green on the current branch:

- `cargo test -p neo-riscv-guest -p neo-riscv-host`
- `NEO_RISCV_HOST_LIB=... dotnet test .../Neo.UnitTests.csproj`
- `scripts/verify-all.sh`

The full `Neo.UnitTests` suite in the C# worktree passes with the Rust host library enabled.
