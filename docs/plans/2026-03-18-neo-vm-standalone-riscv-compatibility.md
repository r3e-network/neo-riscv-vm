# Neo.VM Standalone RISC-V Compatibility Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend verification beyond Neo core so the standalone `neo-vm` repository can execute its existing script corpus against the Rust RISC-V backend.

**Architecture:** Add a compatibility runner inside the standalone `Neo.VM.Tests` project that loads the existing VM JSON corpus, extracts the final expected `HALT` or `FAULT` outcome, and executes the same scripts through the native Rust host library. Keep the original `neo-vm` tests untouched for baseline behavior and add a parallel RISC-V compatibility suite that uses the same inputs.

**Tech Stack:** MSTest in `/home/neo/git/neo-vm`, native Rust FFI from `/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so`, existing VM JSON corpus under `tests/Neo.VM.Tests/Tests`.

### Task 1: Add a failing standalone compatibility test

**Files:**
- Create: `/home/neo/git/neo-vm/tests/Neo.VM.Tests/UT_RiscvVmJson.cs`
- Create: `/home/neo/git/neo-vm/tests/Neo.VM.Tests/Types/RiscvVmRunner.cs`
- Test: `/home/neo/git/neo-vm/tests/Neo.VM.Tests/Neo.VM.Tests.csproj`

**Step 1: Write a minimal failing compatibility test**

Use one existing JSON case with a final `HALT` result, such as `NOP.json`, and assert that the Rust host returns the same final state/result stack as the last JSON step.

**Step 2: Run the narrow test to confirm it fails**

Run: `NEO_RISCV_HOST_LIB=... dotnet test /home/neo/git/neo-vm/tests/Neo.VM.Tests/Neo.VM.Tests.csproj --filter FullyQualifiedName~UT_RiscvVmJson`

Expected: fail because the compatibility runner does not exist yet.

**Step 3: Implement the minimal native runner**

Add:
- native execution-result and stack-item structs
- dynamic native library loading from `NEO_RISCV_HOST_LIB`
- a no-host execution path
- a host-callback execution path for the `TestEngine` syscall behaviors

**Step 4: Re-run the narrow test**

Expected: the new compatibility test passes.

### Task 2: Scale the runner to the standalone VM JSON corpus

**Files:**
- Modify: `/home/neo/git/neo-vm/tests/Neo.VM.Tests/UT_RiscvVmJson.cs`
- Modify: `/home/neo/git/neo-vm/tests/Neo.VM.Tests/Types/RiscvVmRunner.cs`

**Step 1: Reuse the existing JSON corpus**

Walk:
- `/home/neo/git/neo-vm/tests/Neo.VM.Tests/Tests`

For each test case:
- use the last step only
- require final state `HALT` or `FAULT`
- compare final result stack and final fault message

**Step 2: Skip unsupported final `BREAK` cases explicitly**

These remain debugger-state tests requiring per-step invocation-stack reflection, which the current native ABI does not expose.

**Step 3: Run the full standalone compatibility suite**

Run: `NEO_RISCV_HOST_LIB=... dotnet test /home/neo/git/neo-vm/tests/Neo.VM.Tests/Neo.VM.Tests.csproj --filter FullyQualifiedName~UT_RiscvVmJson`

Expected: identify the first real opcode/ABI mismatches against the standalone corpus.

### Task 3: Close the first native compatibility gaps with TDD

**Files:**
- Modify: `/home/neo/git/neo-riscv-vm/crates/neo-riscv-guest/src/lib.rs`
- Modify: `/home/neo/git/neo-riscv-vm/crates/neo-riscv-host/src/lib.rs`
- Modify: `/home/neo/git/neo-riscv-vm/crates/neo-riscv-guest-module/src/main.rs`
- Modify: `/home/neo/git/neo-riscv-vm/crates/neo-riscv-host/tests/runtime.rs`

**Step 1: Promote standalone failures into minimal Rust reproductions where possible**

Examples:
- opcode missing from the Rust guest
- stack-shape mismatch
- syscall bridge mismatch

**Step 2: Verify each reproduction fails before fixing**

Run the narrowest `cargo test` or `dotnet test --filter` that proves the issue.

**Step 3: Implement one minimal fix at a time**

Rebuild the PolkaVM guest blob after each guest-module change.

**Step 4: Re-run Rust tests and standalone compatibility tests**

Expected: steadily reduce standalone compatibility failures without regressing Neo core.

### Task 4: Document the new verification surface

**Files:**
- Modify: `/home/neo/git/neo-riscv-vm/README.md`
- Modify: `/home/neo/git/neo-riscv-vm/scripts/verify-all.sh`

**Step 1: Add the standalone `neo-vm` compatibility command**

Append to unattended verification:
- Rust tests
- Neo core `Neo.UnitTests`
- standalone `neo-vm` compatibility suite

**Step 2: Keep the scope honest**

Document that:
- the compatibility suite currently covers all existing standalone VM JSON cases with final `HALT` or `FAULT` states
- debugger-only final `BREAK` cases require a richer per-step native VM state ABI before they can be validated through the RISC-V backend
