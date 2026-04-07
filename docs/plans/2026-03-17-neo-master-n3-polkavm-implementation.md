# Neo Master-N3 PolkaVM Implementation Plan

> **For Implementer:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first working slice of a `master-n3` C# to Rust to PolkaVM NeoVM execution path without changing existing `ApplicationEngine.Create()` callers.

**Architecture:** C# `neo` keeps chain semantics and injects a provider-backed `RiscvApplicationEngine`. The engine calls a Rust native library, and the Rust host runtime owns a PolkaVM guest that interprets Neo bytecode. The first slice proves engine redirection and trivial script execution, then grows toward syscall and parity coverage.

**Tech Stack:** C# `neo-project/neo` (`master-n3`), .NET 10, Rust stable, PolkaVM, P/Invoke, xUnit, Cargo test

### Task 1: Document the approved architecture

**Files:**
- Create: `docs/plans/2026-03-17-neo-master-n3-polkavm-design.md`
- Create: `docs/plans/2026-03-17-neo-master-n3-polkavm-implementation.md`

**Step 1: Write the design document**

Create the design document with sections for goal, constraints, architecture, host-call ownership, compatibility surface, and phased delivery.

**Step 2: Review the design for drift**

Run: `sed -n '1,240p' docs/plans/2026-03-17-neo-master-n3-polkavm-design.md`
Expected: The document clearly states that `master-n3` C# owns state and Rust+PolkaVM owns bytecode execution.

**Step 3: Commit**

```bash
git add docs/plans/2026-03-17-neo-master-n3-polkavm-design.md docs/plans/2026-03-17-neo-master-n3-polkavm-implementation.md
git commit -m "docs: define neo master-n3 polkavm migration"
```

### Task 2: Add failing C# tests for engine redirection

**Files:**
- Modify: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs`

**Step 1: Write the failing tests**

Add tests that:

- install a temporary provider
- assert `ApplicationEngine.Create(...)` returns `RiscvApplicationEngine`
- load a trivial script such as `PUSH1, RET`
- assert execution delegates to the bridge and halts successfully

**Step 2: Run test to verify it fails**

Run: `dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.SmartContract.UT_ApplicationEngine`
Expected: FAIL because `RiscvApplicationEngine` and the bridge seam do not exist yet.

**Step 3: Commit**

```bash
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter add tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter commit -m "test: specify provider-backed riscv engine behavior"
```

### Task 3: Implement the C# bridge seam

**Files:**
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/IRiscvVmBridge.cs`
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvApplicationEngine.cs`
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvApplicationEngineProvider.cs`
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvExecutionRequest.cs`
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvExecutionResult.cs`
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvBridgeScope.cs`

**Step 1: Write minimal implementation**

Implement:

- a bridge interface for session execution
- a provider that returns `RiscvApplicationEngine`
- an engine subclass that records loaded scripts and overrides `Execute()`
- request/result DTOs for the trivial-script slice
- a disposable helper to install and restore `ApplicationEngine.Provider` in tests

The first implementation only needs to support scripts with no interop and simple stack output.

**Step 2: Run test to verify it passes**

Run: `dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.SmartContract.UT_ApplicationEngine`
Expected: PASS for the new provider-backed engine tests.

**Step 3: Commit**

```bash
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter add src/Neo/SmartContract/RiscV tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter commit -m "feat: add riscv application engine bridge seam"
```

### Task 4: Create the Rust workspace and ABI crate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/neo-riscv-abi/Cargo.toml`
- Create: `crates/neo-riscv-abi/src/lib.rs`
- Create: `crates/neo-riscv-host/Cargo.toml`
- Create: `crates/neo-riscv-host/src/lib.rs`
- Create: `crates/neo-riscv-guest/Cargo.toml`
- Create: `crates/neo-riscv-guest/src/lib.rs`

**Step 1: Write the failing Rust tests**

Add unit tests in `neo-riscv-guest` and `neo-riscv-host` for:

- decoding a trivial Neo script
- executing `PUSH1, RET`
- returning a HALT state and single stack item

**Step 2: Run test to verify it fails**

Run: `cargo test -p neo-riscv-guest -p neo-riscv-host`
Expected: FAIL because interpreter and host bridge code do not exist yet.

**Step 3: Write minimal implementation**

Implement:

- shared ABI enums and structs
- host entry points for trivial execution
- guest interpreter support for `PUSH0..PUSH16` and `RET`

**Step 4: Run test to verify it passes**

Run: `cargo test -p neo-riscv-guest -p neo-riscv-host`
Expected: PASS for the trivial execution slice.

**Step 5: Commit**

```bash
git add Cargo.toml crates/neo-riscv-abi crates/neo-riscv-host crates/neo-riscv-guest
git commit -m "feat: scaffold rust polkavm neo interpreter workspace"
```

### Task 5: Connect the C# bridge to the Rust host library

**Files:**
- Create: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/Native/NativeRiscvVmBridge.cs`
- Modify: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/src/Neo/SmartContract/RiscV/RiscvApplicationEngineProvider.cs`
- Modify: `/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs`

**Step 1: Write the failing integration test**

Replace or extend the fake-bridge test with a native-bridge test guarded by a library-presence check.

**Step 2: Run test to verify it fails**

Run: `dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.SmartContract.UT_ApplicationEngine`
Expected: FAIL because the native bridge glue is still missing.

**Step 3: Write minimal implementation**

Add P/Invoke bindings for the Rust host library and route the provider to it when the library is available.

**Step 4: Run test to verify it passes**

Run: `dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.SmartContract.UT_ApplicationEngine`
Expected: PASS for both fake-bridge and native-bridge trivial execution.

**Step 5: Commit**

```bash
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter add src/Neo/SmartContract/RiscV tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs
git -C /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter commit -m "feat: connect neo application engine to rust riscv host"
```

### Task 6: Verify the first slice end-to-end

**Files:**
- Modify: `README.md`

**Step 1: Document how to run the slice**

Add concise setup and verification notes for building the Rust workspace and running the focused C# tests.

**Step 2: Run verification**

Run: `cargo test`
Expected: PASS in the Rust workspace.

Run: `dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.SmartContract.UT_ApplicationEngine`
Expected: PASS in the C# worktree.

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: describe neo polkavm riscv verification flow"
```
