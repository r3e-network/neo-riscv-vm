# Example Contract Link Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the example RISC-V contracts compile and link through the repo's supported helper/E2E path again.

**Architecture:** Keep contract logic in the example libraries, but give each example an executable-style PolkaVM entry binary so `polkatool link` consumes the same artifact shape as the working guest module. Align `scripts/compile-riscv-contract.sh` and the E2E test with the example-local target directory instead of assuming a root `target/` layout.

**Tech Stack:** Rust, Cargo, PolkaVM tooling, shell E2E scripts.

### Task 1: Lock the regression to the supported helper path

**Files:**
- Modify: `tests/e2e/test-counter.sh`
- Test: `tests/e2e/test-counter.sh`

**Step 1: Point the E2E build at the canonical helper**

Change the counter E2E test to call `./scripts/compile-riscv-contract.sh examples/counter examples/counter/target/counter.polkavm` and keep the existing blob verification checks.

**Step 2: Run the test to verify it fails**

Run:

```bash
./tests/e2e/test-counter.sh
```

Expected: FAIL because the helper cannot find an executable artifact for the example crate yet.

### Task 2: Make example crates produce linkable executable artifacts

**Files:**
- Modify: `examples/counter/Cargo.toml`
- Modify: `examples/counter/src/lib.rs`
- Create: `examples/counter/src/main.rs`
- Modify: `examples/hello-world/Cargo.toml`
- Modify: `examples/hello-world/src/lib.rs`
- Create: `examples/hello-world/src/main.rs`
- Modify: `examples/storage/Cargo.toml`
- Modify: `examples/storage/src/lib.rs`
- Create: `examples/storage/src/main.rs`
- Modify: `examples/nep17-token/Cargo.toml`
- Modify: `examples/nep17-token/src/lib.rs`
- Create: `examples/nep17-token/src/main.rs`
- Modify: `examples/devpack-test/Cargo.toml`
- Modify: `examples/devpack-test/src/lib.rs`
- Create: `examples/devpack-test/src/main.rs`

**Step 1: Refactor the exported entrypoints behind library helpers**

Replace the `#[no_mangle] extern "C" fn invoke(...)` definitions in the libraries with plain exported helper functions that the new binary crates can call.

**Step 2: Add minimal PolkaVM binary roots**

Add `src/main.rs` to each example with the required `no_std`/`no_main` configuration, `_start`/`main` stubs on `riscv32`, a panic handler, and a thin `#[no_mangle] extern "C"` wrapper that delegates into the library helper.

**Step 3: Remove `cdylib` packaging from the example libraries**

Keep the libraries reusable as `rlib`s for tests, but rely on the binary targets for the final executable artifacts.

### Task 3: Make the compile helper use the example-local target directory

**Files:**
- Modify: `scripts/compile-riscv-contract.sh`

**Step 1: Set an explicit target directory under the crate**

Pass `--target-dir "${CRATE_DIR}/target"` to Cargo and resolve the ELF path from that location so the helper works for standalone example crates.

**Step 2: Re-run the helper-backed E2E test**

Run:

```bash
./tests/e2e/test-counter.sh
```

Expected: PASS and produce `examples/counter/target/counter.polkavm`.

### Task 4: Re-validate the integrated flow

**Files:**
- Test: `tests/e2e/run-all.sh`
- Test: `scripts/cross-repo-test.sh`
- Test: `scripts/run-bounded-fuzz.sh`

**Step 1: Re-run the VM E2E suite**

Run:

```bash
./tests/e2e/run-all.sh
```

Expected: PASS.

**Step 2: Re-run cross-repo validation**

Run:

```bash
CORE_DIR=/home/neo/git/neo-riscv/neo-riscv-core NODE_DIR=/home/neo/git/neo-riscv/neo-riscv-node ./scripts/cross-repo-test.sh
```

Expected: advance past the previous E2E blocker and expose only any remaining real failures.

**Step 3: Re-run bounded fuzz smoke**

Run:

```bash
TIME_PER_TARGET=1 RUNS_PER_TARGET=1 FUZZ_SEED=123 ./scripts/run-bounded-fuzz.sh
```

Expected: PASS without crashes.
