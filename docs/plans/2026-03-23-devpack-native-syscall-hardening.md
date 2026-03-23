# Devpack Native Contract + Syscall Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `neo-riscv-devpack` production-credible by fixing syscall ABI correctness, replacing native wrapper stubs with real contract-call wrappers, and validating coverage with deterministic tests.

**Architecture:** Keep C# as syscall/native source-of-truth and harden Rust devpack as a strict wrapper layer. Route native contract methods through `System.Contract.Call` (`hash + method + flags + args`) and decode into typed Rust return values with safe fallbacks. Validate hash/ABI invariants with unit tests and update docs to match actual APIs.

**Tech Stack:** Rust (`no_std` + `alloc`), `neo-riscv-abi::StackValue`, cargo test, existing docs/specs.

### Task 1: Add failing tests for syscall ABI shape and interop hash constants

**Files:**
- Create: `crates/neo-riscv-devpack/tests/api_ids_test.rs`
- Create: `crates/neo-riscv-devpack/tests/syscalls_shape_test.rs`
- Modify: `crates/neo-riscv-devpack/tests/syscalls_test.rs`

**Step 1: Write failing tests for hash constants**

Add assertions that each `api_ids::*` value equals `neo_riscv_abi::interop_hash("...")` for:
- `System.Storage.Get/Put/Delete`
- `System.Contract.Call/Create/Update`
- `System.Runtime.Notify/Log/CheckWitness`
- `System.Crypto.CheckSig`

**Step 2: Write failing tests for contract-call stack shape**

Add a test for a pure helper that builds call stack in Neo order:
1. args array
2. call flags integer
3. method bytes
4. hash bytes

**Step 3: Fix stale syscall test imports**

Update `syscalls_test.rs` to use current storage API (`storage::get/put/delete` or root re-exports) instead of missing `storage_*` symbols.

**Step 4: Run focused tests to confirm failures are meaningful**

Run: `cargo test -p neo-riscv-devpack --test api_ids_test --test syscalls_shape_test --test syscalls_test -v`
Expected: shape/hash tests fail before implementation changes; stale symbol compile errors disappear after test update.

### Task 2: Refactor syscall wrappers for ABI correctness

**Files:**
- Modify: `crates/neo-riscv-devpack/src/syscalls.rs`
- Modify: `crates/neo-riscv-devpack/src/api_ids.rs`

**Step 1: Implement call-flag aware contract call path**

Add:
- `pub const CALL_FLAGS_ALL: u8 = 0x0f`
- `pub fn contract_call_with_flags(hash: &[u8], method: &str, call_flags: u8, args: &[StackValue]) -> StackValue`
- `pub fn build_contract_call_stack(...) -> Vec<StackValue>` (for deterministic testing)

Make `contract_call` call `contract_call_with_flags(..., CALL_FLAGS_ALL, ...)`.

**Step 2: Correct syscall ID definitions**

Use `interop_hash`-aligned values in `api_ids.rs` and add brief comments linking each constant to the interop name.

**Step 3: Keep runtime/crypto wrappers behavior-stable**

Preserve `false` / `Null` fallback behavior on host-call errors to remain safe in non-chain/unit-test contexts.

**Step 4: Run focused tests**

Run: `cargo test -p neo-riscv-devpack --test api_ids_test --test syscalls_shape_test -v`
Expected: all pass.

### Task 3: Replace native stubs with real native-contract wrappers

**Files:**
- Modify: `crates/neo-riscv-devpack/src/native/mod.rs`
- Modify: `crates/neo-riscv-devpack/src/native/contract_management.rs`
- Modify: `crates/neo-riscv-devpack/src/native/crypto_lib.rs`
- Modify: `crates/neo-riscv-devpack/src/native/gas_token.rs`
- Modify: `crates/neo-riscv-devpack/src/native/ledger.rs`
- Modify: `crates/neo-riscv-devpack/src/native/neo_token.rs`
- Modify: `crates/neo-riscv-devpack/src/native/oracle.rs`
- Modify: `crates/neo-riscv-devpack/src/native/policy.rs`
- Modify: `crates/neo-riscv-devpack/src/native/role_management.rs`
- Modify: `crates/neo-riscv-devpack/src/native/std_lib.rs`
- Create: `crates/neo-riscv-devpack/src/native/notary.rs`
- Create: `crates/neo-riscv-devpack/src/native/treasury.rs`

**Step 1: Add shared native-call helpers in `native/mod.rs`**

Implement:
- hash constants for 11 native contracts (from Neo UnitTest native state snapshots)
- generic `call_native(hash, method, args)`
- typed extractors (`as_bool`, `as_i64`, `as_u32`, `as_bytes`, etc.)

**Step 2: Refactor existing modules to use `call_native`**

Each wrapper must call canonical method names (`balanceOf`, `transfer`, `getCandidates`, etc.) and convert results to Rust types with safe defaults.

**Step 3: Add missing Notary and Treasury modules**

Expose minimum practical wrappers:
- Notary: `balance_of`, `expiration_of`, `lock_deposit_until`, `withdraw`, `get_max_not_valid_before_delta`, `set_max_not_valid_before_delta`
- Treasury: `verify` (+ callbacks optional, usually not external-call use)

**Step 4: Register new modules**

Export new modules from `native/mod.rs` and ensure compile visibility through crate root.

**Step 5: Run crate tests**

Run: `cargo test -p neo-riscv-devpack -v`
Expected: all tests pass and unused-variable noise is significantly reduced.

### Task 4: Align docs and examples with real API surface

**Files:**
- Modify: `docs/native-contracts.md`
- Modify: `docs/syscall-api.md`
- Modify: `docs/contract-examples.md`
- Modify: `README.md` (devpack/API references only if needed)

**Step 1: Remove stale naming**

Replace `storage_get/storage_put/storage_delete` naming with actual Rust API (`storage::get/put/delete` and re-exports).

**Step 2: Document all 11 native contracts**

List supported contracts and wrapper scope, including Notary/Treasury coverage and limitations.

**Step 3: Document contract call semantics clearly**

State exact `System.Contract.Call(hash, method, flags, args)` stack order and default call-flags policy.

**Step 4: Validate docs consistency**

Run: `rg -n "storage_get|storage_put|storage_delete|9 native|8 native" docs README.md`
Expected: no stale references.

### Task 5: Full verification and quality gate

**Files:**
- Modify only as needed based on failures from verification commands

**Step 1: Rust workspace verification**

Run: `cargo test --workspace --all-targets`

**Step 2: Formatting and lint verification**

Run:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

**Step 3: Optional integration scripts (best effort)**

Run if environment allows:
- `tests/e2e/run-all.sh`
- `scripts/verify-all.sh`

**Step 4: Collect evidence**

Capture pass/fail results and any remaining risks (toolchain/env/external node dependencies).

### Task 6: Final review and cleanup

**Files:**
- Modify any touched files for final polish only

**Step 1: Consistency pass**

Verify naming, module exports, docs, and tests are mutually consistent.

**Step 2: Remove dead code/imports**

Eliminate stale imports and placeholder comments that no longer match behavior.

**Step 3: Final status report**

Summarize:
- what was fixed
- what was validated
- what remains intentionally out-of-scope
