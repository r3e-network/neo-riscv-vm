# Cross-Repo Compatibility Audit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand Neo-on-RISC-V validation from targeted regressions to a documented cross-repo proof set covering the VM workspace, core adapter integration, devpack backend execution, and the highest-value missing fuzz/parity path.

**Architecture:** Treat compatibility as a matrix, not a slogan. The VM repo remains the execution substrate, `neo-riscv-core` proves application-engine and native-contract behavior through the adapter, and `neo-riscv-devpack` proves compiler/runtime backend behavior. Existing targeted fixes stay in place; this audit adds broader unattended verification, captures any new regressions with minimal TDD loops, and closes the largest current blind spot by moving whole-system parity fuzzing toward a host-backed harness instead of more guest-only stress tests.

**Tech Stack:** Rust workspace tests, C# `dotnet test`, PolkaVM guest blob regeneration, RISC-V backend caches under `/tmp`, libFuzzer-style fuzz entrypoints, existing callback codec and host bridge code.

### Task 1: Rebuild the validation matrix and re-run the broad suites

**Files:**
- Modify: `docs/plans/2026-04-08-cross-repo-compatibility-audit.md`
- Reference: `scripts/regenerate-guest-blob.sh`
- Reference: `compat/Neo.Riscv.Adapter/NativeRiscvVmBridge.cs`
- Reference: `/home/neo/git/neo-riscv/neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj`
- Reference: `/home/neo/git/neo-riscv/neo-riscv-devpack/tests/Neo.Compiler.CSharp.UnitTests/Neo.Compiler.CSharp.UnitTests.csproj`
- Reference: `/home/neo/git/neo-riscv/neo-riscv-devpack/tests/Neo.SmartContract.Testing.UnitTests/Neo.SmartContract.Testing.UnitTests.csproj`

**Step 1: Rebuild the guest artifact**

Run:

```bash
bash /home/neo/git/neo-riscv/neo-riscv-vm/scripts/regenerate-guest-blob.sh
```

Expected: `crates/neo-riscv-guest-module/guest.polkavm` matches the current guest-module source.

**Step 2: Clear stale RISC-V compiler caches before backend verification**

Run:

```bash
rm -rf /tmp/riscv-test-output/riscv /tmp/neo-riscv-test-contracts/riscv
```

Expected: subsequent compiler/backend tests cannot reuse stale `.polkavm` outputs.

**Step 3: Run the broad VM suite**

Run:

```bash
cargo test --workspace --all-targets
```

Expected: PASS, or a concrete failing subsystem to investigate with `superpowers:systematic-debugging`.

**Step 4: Run the broad core adapter suite**

Run:

```bash
NEO_RISCV_HOST_LIB=/home/neo/git/neo-riscv/neo-riscv-vm/target/release/libneo_riscv_host.so \
dotnet test /home/neo/git/neo-riscv/neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj
```

Expected: PASS, or a reduced failing slice that identifies an uncovered compatibility gap.

**Step 5: Run the broad devpack suites**

Run:

```bash
dotnet test /home/neo/git/neo-riscv/neo-riscv-devpack/tests/Neo.Compiler.CSharp.UnitTests/Neo.Compiler.CSharp.UnitTests.csproj
dotnet test /home/neo/git/neo-riscv/neo-riscv-devpack/tests/Neo.SmartContract.Testing.UnitTests/Neo.SmartContract.Testing.UnitTests.csproj
```

Expected: PASS for both backends, or a narrowed failing contract/runtime behavior to investigate.

### Task 2: Fix any newly exposed regression with strict TDD loops

**Files:**
- Modify: exact failing Rust or C# source file identified by Task 1
- Modify: exact failing test file identified by Task 1 only if a regression test is missing

**Step 1: Preserve the smallest failing reproduction**

Run the narrowest `cargo test` or `dotnet test --filter` command that still fails.

Expected: a stable, single-problem reproduction.

**Step 2: Add the smallest regression test only when the failure is not already covered**

Example shape:

```rust
#[test]
fn storage_round_trip_survives_iterator_callback() {
    // minimal reproduction derived from Task 1 failure
}
```

or

```csharp
[TestMethod]
public void RuntimeBurnGasMatchesNeoVmAccounting()
{
    // minimal reproduction derived from Task 1 failure
}
```

**Step 3: Re-run the narrow test to verify it fails for the right reason**

Expected: FAIL due to the proven behavioral gap, not environment setup.

**Step 4: Apply the minimal fix**

Expected: preserve existing architecture, avoid speculative refactors, and keep unrelated dirty-worktree changes intact.

**Step 5: Re-run the narrow test and the nearest containing suite**

Expected: PASS for both the targeted regression and its parent suite slice.

### Task 3: Land a real host-backed whole-system parity fuzz path

**Files:**
- Modify: `fuzz/Cargo.toml`
- Modify: `fuzz/README.md`
- Modify: `docs/plans/2026-04-08-whole-system-fuzz-audit.md`
- Create: `fuzz/src/whole_system_parity.rs`
- Reference: `Cargo.toml`
- Reference: `Cargo.lock`
- Reference: `crates/neo-riscv-host/src/lib.rs`
- Reference: `crates/neo-riscv-host/tests/parity.rs`

**Step 1: Reproduce the current packaging blocker**

Run:

```bash
cargo test --manifest-path /home/neo/git/neo-riscv/neo-riscv-vm/fuzz/Cargo.toml --bin whole_system_parity
```

Expected: FAIL until the standalone fuzz package can consume the host/runtime dependency set.

**Step 2: Align the fuzz package dependency resolution with the root workspace**

Expected: `neo-riscv-host` becomes consumable without downgrading the goal to another guest-only fuzzer.

**Step 3: Add unit tests that compare direct guest and host-path executions for fixed scenarios**

Expected coverage:
- syscall callback order
- storage put/get/delete round-trips
- readonly/local-context handling
- nested compound return-value codec fidelity

**Step 4: Implement the minimal whole-system parity harness**

Run:

```bash
cargo test --manifest-path /home/neo/git/neo-riscv/neo-riscv-vm/fuzz/Cargo.toml --bin whole_system_parity
```

Expected: PASS.

**Step 5: Run a bounded smoke fuzz pass**

Run:

```bash
cargo build --manifest-path /home/neo/git/neo-riscv/neo-riscv-vm/fuzz/Cargo.toml --bin whole_system_parity --release
/home/neo/git/neo-riscv/neo-riscv-vm/fuzz/target/release/whole_system_parity -runs=100
```

Expected: no crashes and no parity assertion failures.

### Task 4: Produce an evidence-backed readiness summary

**Files:**
- Modify: `docs/plans/2026-04-08-cross-repo-compatibility-audit.md`
- Modify: `fuzz/README.md`

**Step 1: Record the exact commands run and their outcomes**

Expected: no "100% compatible" claim without corresponding suite evidence.

**Step 2: Separate proven coverage from remaining gaps**

Expected categories:
- proven green suites
- fixed regressions
- bounded fuzz coverage
- known blind spots or unrun suites

**Step 3: Re-run the final verification commands before reporting success**

Expected: the final report is backed by current outputs, not earlier assumptions.
