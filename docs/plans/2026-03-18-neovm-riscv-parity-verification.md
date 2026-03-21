# NeoVM RISC-V Parity Verification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure the NeoVM-on-RISC-V path matches existing NeoVM behavior closely enough to pass the existing Neo VM unit tests and the broader `Neo.UnitTests` suite on Neo `master-n3`.

**Architecture:** The Rust PolkaVM host remains the execution boundary that Neo C# calls through, while the checked-in `guest.polkavm` blob remains the deployed guest artifact. Verification must prove that the embedded blob matches the latest guest-module source and that syscall/error/fee propagation through the guest boundary preserves NeoVM semantics.

**Tech Stack:** Rust workspace (`neo-riscv-host`, `neo-riscv-guest`, `neo-riscv-guest-module`), PolkaVM interpreter backend, C# Neo `master-n3` worktree, `dotnet test`, `cargo test`, `polkatool`.

### Task 1: Confirm the active runtime artifact and reproduce failures

**Files:**
- Inspect: `crates/neo-riscv-guest-module/src/main.rs`
- Inspect: `crates/neo-riscv-guest-module/guest.polkavm`
- Inspect: `crates/neo-riscv-host/src/lib.rs`
- Test: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj`

**Step 1: Verify whether guest-module source and embedded blob are in sync**

Run: `stat -c '%y %n' crates/neo-riscv-guest-module/src/main.rs crates/neo-riscv-guest-module/guest.polkavm`
Expected: If `src/main.rs` is newer, the blob is stale and the latest guest changes are not active.

**Step 2: Run focused Rust verification**

Run: `cargo test -p neo-riscv-guest -p neo-riscv-host`
Expected: Rust tests pass or expose the current guest/host regression surface.

**Step 3: Run focused Neo VM verification**

Run: `NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.VM`
Expected: Existing Neo VM tests pass, or the failing cases identify remaining parity gaps.

### Task 2: Update the deployed guest artifact using TDD constraints

**Files:**
- Modify: `crates/neo-riscv-guest-module/src/main.rs`
- Modify: `crates/neo-riscv-guest-module/guest.polkavm`
- Inspect: `scripts/regenerate-guest-blob.sh`
- Test: `crates/neo-riscv-host/src/lib.rs`

**Step 1: Keep the failing reproduction in hand**

Run: the focused failing test command from Task 1.
Expected: The failure remains reproducible before artifact regeneration or code changes.

**Step 2: Apply the minimal root-cause fix**

Fixes must stay limited to the guest boundary defect actually proven by Task 1:
- instruction charging across the guest-module path
- syscall error propagation across the guest-module path
- any artifact drift preventing the fix from being executed

**Step 3: Regenerate the checked-in PolkaVM blob**

Run: `scripts/regenerate-guest-blob.sh`
Expected: `crates/neo-riscv-guest-module/guest.polkavm` is rebuilt from the current guest-module source.

**Step 4: Re-run the exact failing tests**

Run: the same focused Rust or `dotnet test --filter` commands that previously failed.
Expected: The tests now pass for the exact failure that drove the fix.

### Task 3: Close any remaining parity gaps with minimal regression coverage

**Files:**
- Modify: `crates/neo-riscv-host/src/lib.rs`
- Modify: `crates/neo-riscv-guest/src/lib.rs`
- Modify: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs`
- Modify: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/SmartContract/UT_ApplicationEngineProvider.cs`

**Step 1: Add or tighten the smallest regression test only if an uncovered failure remains**

Expected coverage targets:
- `Runtime.BurnGas` / `GasLeft` accounting
- host fault-message propagation
- read-only storage context fault behavior
- contract call permission faults

**Step 2: Verify the new test fails for the right reason**

Run: the narrowest possible `cargo test` or `dotnet test --filter` command.
Expected: The test fails because of the missing behavior, not because of bad setup.

**Step 3: Implement the minimal fix and re-run the narrow test**

Expected: The new regression test passes without broad unrelated changes.

### Task 4: Full unattended verification and documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `scripts/verify-all.sh`

**Step 1: Add a single unattended verification entry point if missing**

Run target should include:
- `cargo test -p neo-riscv-guest -p neo-riscv-host`
- `NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj`

**Step 2: Run the full verification**

Expected: Rust tests pass and the complete Neo `Neo.UnitTests` suite passes against the RISC-V host bridge.

**Step 3: Update docs to reflect the canonical production-ready verification path**

Expected: The docs make it clear that the checked-in `guest.polkavm` is authoritative and must be regenerated whenever guest-module source changes.
