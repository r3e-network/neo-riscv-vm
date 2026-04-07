# NeoVM EQUAL Semantics Parity Implementation Plan

> **For Implementer:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the RISC-V guest interpreter match NeoVM `EQUAL`/`NOTEQUAL` semantics for compound stack items without changing upstream NeoVM tests or external tooling.

**Architecture:** Keep the existing interpreter and ABI stable. Add a focused script-level regression in the Rust guest tests, then replace the current structural `deep_equal` use in `EQUAL`/`NOTEQUAL` with NeoVM-compatible equality semantics: identity for `Array`/`Map`/`Buffer`, structural equality for `Struct`, and value equality for primitive types and byte strings.

**Tech Stack:** Rust, cargo test, copied NeoVM JSON compatibility suite, .NET test runner.

### Task 1: Reproduce and capture the failing behavior

**Files:**
- Inspect: `compat/Neo.VM.Riscv.Tests/Corpus/Tests/OpCodes/BitwiseLogic/EQUAL.json`
- Inspect: `crates/neo-riscv-guest/src/lib.rs`
- Inspect: `crates/neo-riscv-guest/src/helpers.rs`

**Step 1: Run the copied compatibility file**

Run: `NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so NEO_RISCV_VM_JSON_FILTER=/OpCodes/BitwiseLogic/EQUAL.json dotnet test /home/neo/git/neo-riscv-vm/compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj --filter FullyQualifiedName~TestCopiedNeoVmJsonFinalStates`

Expected: `Array=false`, `Map=false`, and `Buffer=false` cases fail while the other `EQUAL` cases stay green.

### Task 2: Add the Rust regression first

**Files:**
- Modify: `crates/neo-riscv-guest/tests/interpreter.rs`

**Step 1: Write focused failing tests**

Add script-level tests for:
- distinct empty arrays compare `false`
- duplicate array reference compares `true`
- distinct empty maps compare `false`
- duplicate map reference compares `true`
- distinct equal-content buffers compare `false`
- duplicated buffer reference compares `true`
- distinct empty structs compare `true`

**Step 2: Run the targeted Rust guest tests**

Run: `cargo test -p neo-riscv-guest executes_equal`

Expected: the new tests fail before implementation because the current guest interpreter still uses structural equality for all compound values.

### Task 3: Implement the minimal runtime fix

**Files:**
- Modify: `crates/neo-riscv-guest/src/helpers.rs`
- Modify: `crates/neo-riscv-guest/src/lib.rs`

**Step 1: Add NeoVM-compatible equality helper**

Implement a helper that:
- compares integers, big integers, booleans, byte strings, interop handles, iterators, and null by value
- compares arrays/maps/buffers by compound identity
- compares structs structurally and recursively

**Step 2: Switch `EQUAL`/`NOTEQUAL` to the new helper**

Keep `deep_equal` available only where structural comparison is still needed internally.

**Step 3: Re-run the targeted Rust guest tests**

Run: `cargo test -p neo-riscv-guest executes_equal`

Expected: the new tests pass.

### Task 4: Verify compatibility and broader safety

**Files:**
- Verify only

**Step 1: Run the copied NeoVM compatibility file again**

Run: `NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so NEO_RISCV_VM_JSON_FILTER=/OpCodes/BitwiseLogic/EQUAL.json dotnet test /home/neo/git/neo-riscv-vm/compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj --filter FullyQualifiedName~TestCopiedNeoVmJsonFinalStates`

Expected: green.

**Step 2: Run the Rust suites affected by the change**

Run: `cargo test -p neo-riscv-guest -p neo-riscv-host`

Expected: green.

**Step 3: Run the Neo core VM subset**

Run: `NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv-vm/target/debug/libneo_riscv_host.so dotnet test /home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj --filter FullyQualifiedName~Neo.UnitTests.VM`

Expected: green.
