# Whole-System Fuzz Audit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a whole-system NeoVM-on-RISC-V fuzz target that catches parity, syscall marshalling, and native-contract boundary regressions that the current guest-local fuzzers do not cover.

**Architecture:** Keep the existing `fuzz/` package as the entry point for fast local and CI fuzzing, but add a structured whole-system target instead of another raw-byte interpreter stressor. The new target should drive the same generated scenario through two execution paths: direct guest interpretation and the real PolkaVM host path, then compare callback traces and normalized execution results. Native-contract coverage should be layered on top of the same scenario model once the fuzz package can depend on the host/runtime path cleanly.

**Tech Stack:** Rust, libFuzzer, `neo-riscv-guest`, `neo-riscv-abi`, `neo-riscv-host`, existing callback codec and host callback APIs.

## Current Coverage

- `fuzz/src/opcode_seq.rs`
  - Fuzzes random opcode sequences against `neo_riscv_guest::interpret_with_stack_and_syscalls`.
  - Uses a `NoOpSyscall` provider and only checks `Halt`/`Fault` shape.
  - Misses host callback marshalling, PolkaVM execution, callback codec, gas reconciliation, and syscall semantics.

- `fuzz/src/type_convert.rs`
  - Exercises a narrow set of conversion and `ISTYPE`-style opcodes in the direct guest path.
  - Misses whole-system parity, syscall boundaries, and native-contract interaction.

- `fuzz/src/stack_ops.rs`
  - Exercises stack-manipulation opcodes in the direct guest path.
  - Misses callback argument ordering and any adapter-visible behavior.

- `fuzz/src/exception_handling.rs`
  - Exercises `TRY`/`THROW`/`ENDTRY` control flow only in the guest interpreter.
  - Misses host-path exception transport and adapter/native error rehydration.

- `fuzz/src/mem_op.rs`
  - Exercises splice/memory opcodes only in the guest interpreter.
  - Misses any interop or callback serialization coverage.

- `fuzz/src/syscall_fuzz.rs`
  - The closest existing target to the real risk area.
  - Still only runs the direct guest interpreter.
  - Uses synthetic random syscall returns instead of checking exact parity against the real host callback path.
  - Does not verify callback trace equality, argument ordering, nested return value round-trips, storage-context handling, or result normalization across the host boundary.

- `crates/neo-riscv-host/tests/opcode_matrix/fuzzing.rs`
  - Property-tests only `PUSHINT8` and `PUSHDATA1` through `neo_riscv_host::execute_script`.
  - Useful sanity coverage, but too small to represent whole-system parity.

## What The Current Suite Misses

1. Direct guest versus PolkaVM host-path parity for the same script and syscall model.
2. Callback codec fidelity for nested `Array`, `Struct`, and `Map` returns.
3. Syscall argument truncation and ordering across the host boundary.
4. Storage-context and readonly-context behavior under structured multi-syscall workloads.
5. Cross-checking callback traces, not just final result shape.
6. Native-contract direct execution under generated syscall/native scenarios.
7. Adapter-visible regressions such as script-hash shape, script-container shape, verification-message building, and gas-accounting drift.

## Highest-Value Next Step

Implement a new `fuzz/src/whole_system_parity.rs` target that generates structured syscall-centric scenarios and runs them through:

1. `neo_riscv_guest::interpret_with_stack_and_syscalls`
2. `neo_riscv_host::execute_script_with_host_and_stack`

Both paths must use the same deterministic syscall model and must record the same callback trace. The harness should compare:

- callback sequence (`api`, `ip`, normalized args)
- final VM state
- final stack contents
- normalized fault class/message when the scenario is intentionally faulting

This is the best next step because it covers the real guest/host boundary while staying inside `neo-riscv-vm`. It is also the smallest meaningful bridge from the current guest-local fuzzers to the adapter/native-contract problem space.

## Current Blocker

The standalone `fuzz/Cargo.toml` package is its own workspace. Adding `neo-riscv-host` directly causes a fresh dependency resolution for `polkavm = 0.32.0`, which is yanked on crates.io, so the harness cannot currently be landed in `fuzz/` without either:

1. teaching the fuzz workspace how to consume the same locked PolkaVM dependency set as the root workspace, or
2. moving the whole-system fuzz target into the root workspace test/fuzz surface instead of the standalone fuzz package.

Until that packaging issue is resolved, another guest-only fuzzer would add less value than fixing the dependency path and landing the parity harness properly.

### Task 1: Make the standalone fuzz package able to consume the host path

**Files:**
- Modify: `fuzz/Cargo.toml`
- Modify: `fuzz/Cargo.lock`
- Reference: `Cargo.toml`
- Reference: `Cargo.lock`

**Step 1: Write the failing dependency-integration test**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity
```

Expected: dependency resolution fails when `neo-riscv-host` is added because the standalone fuzz package cannot resolve the yanked `polkavm = 0.32.0`.

**Step 2: Make the fuzz package resolve the same host/runtime dependency set as the root workspace**

- Do not patch around the problem by downgrading the harness to a guest-only target.
- Reuse the root workspace dependency resolution strategy or lock state.
- Keep `cargo test --manifest-path fuzz/Cargo.toml --lib` and `cargo build --manifest-path fuzz/Cargo.toml --bins` working.

**Step 3: Re-run the failing command**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity
```

Expected: the new bin compiles and its unit tests run.

### Task 2: Add the whole-system parity syscall harness

**Files:**
- Create: `fuzz/src/whole_system_parity.rs`
- Modify: `fuzz/Cargo.toml`
- Modify: `fuzz/README.md`

**Step 1: Write the failing tests**

Add unit tests that:

- build a structured scenario from a fixed seed
- execute it through both direct guest and host paths
- assert callback-trace equality
- assert exact stack/state parity for a storage round-trip and a compound-value-return scenario

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity
```

Expected: tests fail because the harness does not yet exist.

**Step 3: Write the minimal implementation**

Implement:

- a deterministic scenario generator using a constrained operation set
- a shared syscall model with in-memory storage and recorded callback trace
- a direct guest runner
- a host-path runner
- result normalization and parity assertions

Keep the supported syscall set intentionally small at first:

- `System.Runtime.Platform`
- `System.Runtime.GetTrigger`
- `System.Runtime.GetNetwork`
- `System.Runtime.CheckWitness`
- `System.Storage.GetContext`
- `System.Storage.GetReadOnlyContext`
- `System.Storage.AsReadOnly`
- `System.Storage.Get`
- `System.Storage.Put`
- `System.Storage.Delete`
- `System.Storage.Local.Get`
- `System.Storage.Local.Put`
- `System.Storage.Local.Delete`
- one zero-arg syscall that returns a nested compound value for codec coverage

**Step 4: Run the tests to verify they pass**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity
```

Expected: PASS.

**Step 5: Add a smoke fuzz invocation**

Run:

```bash
cargo build --manifest-path fuzz/Cargo.toml --bin whole_system_parity --release
./fuzz/target/release/whole_system_parity -runs=100
```

Expected: exits without crashes or parity failures.

### Task 3: Extend the scenario model to native-contract coverage

**Files:**
- Modify: `fuzz/src/whole_system_parity.rs`
- Reference: `crates/neo-riscv-host/src/lib.rs`
- Reference: `crates/neo-riscv-host/tests/runtime.rs`
- Reference: `compat/Neo.Riscv.Adapter/NativeRiscvVmBridge.cs`

**Step 1: Write the failing native-contract regression tests**

Add tests for a fixed binary/method corpus that:

- executes via `execute_native_contract`
- uses the same deterministic syscall model
- asserts callback-trace and result-shape parity against an equivalent scripted scenario when applicable

**Step 2: Run to verify failure**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity native_contract
```

Expected: FAIL until native-contract scenarios are wired in.

**Step 3: Implement minimal native-contract corpus support**

- Start with a tiny fixed corpus, not arbitrary binaries.
- Reuse already-generated trusted binaries or checked-in corpus assets.
- Record callback traces and normalize output exactly like the script path.

**Step 4: Re-run the tests**

Run:

```bash
cargo test --manifest-path fuzz/Cargo.toml --bin whole_system_parity native_contract
```

Expected: PASS.

### Task 4: Add adapter-facing parity seeds and regression capture

**Files:**
- Create: `fuzz/corpus/whole_system_parity/`
- Modify: `fuzz/README.md`
- Reference: `neo-riscv-core/tests/Neo.UnitTests/SmartContract/UT_ApplicationEngine.cs`

**Step 1: Seed the corpus with known-risk scenarios**

Include fixed seeds for:

- runtime script-hash syscalls
- burn-gas accounting
- verification-message building
- `verifyWithECDsa` after array packing
- storage readonly-context transitions
- multisig-account creation stack packing

**Step 2: Run a time-boxed smoke**

Run:

```bash
./fuzz/target/release/whole_system_parity -max_total_time=60 fuzz/corpus/whole_system_parity/
```

Expected: corpus runs cleanly and any future regression is reproducible from a committed seed.

