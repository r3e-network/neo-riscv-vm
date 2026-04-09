# Example Contract E2E Link Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the example contracts compile into PolkaVM-linkable executables and restore the `tests/e2e` path used by the repo orchestrators.

**Architecture:** Keep example contract logic in `src/lib.rs` for unit tests and reuse, but add binary entrypoints so Cargo emits executable artifacts for PolkaVM. Make the E2E scripts call the canonical `scripts/compile-riscv-contract.sh` path instead of hand-building and linking a `.elf` that no longer matches the intended artifact shape.

**Tech Stack:** Bash, Cargo nightly, PolkaVM toolchain, Rust `no_std` binaries

### Task 1: Reproduce The Broken Link Path

**Files:**
- Modify: `tests/e2e/test-counter.sh`
- Verify: `scripts/compile-riscv-contract.sh`

**Step 1: Write the failing test**

Change `tests/e2e/test-counter.sh` to compile `examples/counter` via `./scripts/compile-riscv-contract.sh examples/counter examples/counter/target/counter.polkavm`.

**Step 2: Run test to verify it fails**

Run: `cd /home/neo/git/neo-riscv/neo-riscv-vm && ./tests/e2e/test-counter.sh`
Expected: fail because `scripts/compile-riscv-contract.sh` cannot find the executable artifact for `examples/counter`.

### Task 2: Emit Executable Example Artifacts

**Files:**
- Modify: `examples/counter/Cargo.toml`
- Modify: `examples/counter/src/lib.rs`
- Create: `examples/counter/src/main.rs`
- Repeat same pattern for:
  `examples/hello-world`, `examples/storage`, `examples/nep17-token`, `examples/devpack-test`

**Step 1: Write the minimal implementation**

For each example package:
- keep contract logic in `src/lib.rs`
- remove exported `#[no_mangle] extern "C"` entrypoints from the library
- add `src/main.rs` with PolkaVM executable entrypoints (`_start`, `main`, exported `invoke`)
- keep the library target as `rlib`
- declare or default the binary target name so `scripts/compile-riscv-contract.sh` can locate it

**Step 2: Run the targeted test**

Run: `cd /home/neo/git/neo-riscv/neo-riscv-vm && ./scripts/compile-riscv-contract.sh examples/counter /tmp/counter-test.polkavm`
Expected: success and `/tmp/counter-test.polkavm` exists.

### Task 3: Harden The Canonical Compile Script

**Files:**
- Modify: `scripts/compile-riscv-contract.sh`

**Step 1: Improve target resolution**

Replace the underscore-only artifact guess with exact bin-target resolution from Cargo metadata or an equivalent deterministic lookup. Fail clearly when a package has no bin target.

**Step 2: Verify with a hyphenated example**

Run: `cd /home/neo/git/neo-riscv/neo-riscv-vm && ./scripts/compile-riscv-contract.sh examples/hello-world /tmp/hello-world-test.polkavm`
Expected: success and the linked blob exists.

### Task 4: Re-Run Orchestrators

**Files:**
- Verify only

**Step 1: Run E2E**

Run: `cd /home/neo/git/neo-riscv/neo-riscv-vm && ./tests/e2e/run-all.sh`
Expected: pass.

**Step 2: Run cross-repo validation**

Run: `cd /home/neo/git/neo-riscv/neo-riscv-vm && CORE_DIR=/home/neo/git/neo-riscv/neo-riscv-core NODE_DIR=/home/neo/git/neo-riscv/neo-riscv-node ./scripts/cross-repo-test.sh`
Expected: advance past VM and E2E validation without the previous linker failure.
