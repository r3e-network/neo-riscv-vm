# Production Readiness Hardening Implementation Plan

> **For Implementer:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Drive the remaining RISC-V integration failures to zero while keeping the adapter and compatibility logic isolated inside `neo-riscv-vm`.

**Architecture:** The Rust guest/host and the C# adapter remain the only places aware of the RISC-V execution boundary. Existing Neo core and node callers continue using `ApplicationEngine`, `StackItem`, manifest metadata, and plugin loading exactly as before; parity gaps are closed inside the bridge, compatibility dispatch, and test harness layers.

**Tech Stack:** Rust workspace (`cargo test`, `cargo clippy`), C# adapter/tests (`dotnet test`), Neo `../neo-riscv-core` integration tests, copied NeoVM JSON corpus, PolkaVM host bridge.

### Task 1: Establish the live failure set

**Files:**
- Modify: `scripts/verify-all.sh`
- Test: `../neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj`
- Test: `compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj`

**Step 1: Run the smallest complete verification surface**

Run: `scripts/verify-all.sh`
Expected: Rust tests, local compatibility tests, and adapter tests pass.

**Step 2: Run the full current Neo integration suite as the source of truth**

Run: `NEO_RUN_NEO_UNITTESTS=1 scripts/verify-all.sh`
Expected: Capture the exact remaining failing tests against `../neo-riscv-core`.

**Step 3: Record the failures before code changes**

For each failing integration test:
- note the fully qualified test name
- note the asserted vs actual behavior
- map it to the likely owning layer (`NativeRiscvVmBridge`, `RiscvApplicationEngine`, Rust host, Rust guest, or packaging/provider resolution)

### Task 2: Fix one failing behavior at a time

**Files:**
- Modify: `compat/Neo.Riscv.Adapter/*.cs`
- Modify: `crates/neo-riscv-host/src/*.rs`
- Modify: `crates/neo-riscv-guest/src/*.rs`
- Test: `compat/Neo.Riscv.Adapter.Tests/*.cs`
- Test: `../neo-riscv-core/tests/Neo.UnitTests/**/*.cs`

**Step 1: Pick one failing test as the repro**

Run: `dotnet test ../neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~<failing-test>`
Expected: One clear failure.

**Step 2: Add or tighten the narrowest local regression test only if needed**

Preferred local test homes:
- adapter/interop routing: `compat/Neo.Riscv.Adapter.Tests`
- host/guest ABI behavior: Rust tests in `crates/neo-riscv-host/tests` or `crates/neo-riscv-guest/tests`

**Step 3: Verify RED**

Run the new or existing narrow failing test before the implementation change.
Expected: It fails for the proven missing behavior.

**Step 4: Implement the smallest fix**

Allowed scope:
- bridge-side call flags / dynamic load semantics
- storage / iterator / interop item translation
- native syscall routing and error propagation
- fee / gas / fault-message propagation
- contract type / manifest compatibility behavior

**Step 5: Verify GREEN**

Run:
- the narrow failing integration test
- the relevant local regression test

Expected: both pass.

### Task 3: Keep the verification surface fast and trustworthy

**Files:**
- Modify: `compat/Neo.VM.Riscv.Tests/*.cs`
- Modify: `scripts/verify-all.sh`
- Modify: `README.md`

**Step 1: Ensure defaults stay fast**

Keep default local verification on:
- Rust tests
- local JSON compatibility tests
- adapter tests

Keep the large Neo integration suite opt-in unless explicitly requested.

**Step 2: Ensure the full path stays available**

Run: `NEO_RUN_NEO_UNITTESTS=1 scripts/verify-all.sh`
Expected: It runs the full external suite with the release host library.

### Task 4: Close out with evidence

**Files:**
- Modify: `README.md`
- Modify: `docs/plans/2026-03-22-production-readiness-hardening.md`

**Step 1: Run final verification**

Run:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets`
- `cargo test -p neo-riscv-guest -p neo-riscv-host`
- `dotnet test compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj`
- `dotnet test compat/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj`
- `NEO_RUN_NEO_UNITTESTS=1 scripts/verify-all.sh`

Expected: zero failures.

**Step 2: Update docs only after green**

Document:
- the canonical fast verification commands
- the canonical full integration verification command
- any remaining environment assumptions
