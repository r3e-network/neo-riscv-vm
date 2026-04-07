# Node Local Core And StateRoot Validation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ensure `neo-riscv-node` builds entirely against the local `neo-riscv-core` fork where required for RISC-V support, then validate end-to-end state-root parity against a reference Neo node.

**Architecture:** Keep the dependency graph explicit and reproducible: `neo-riscv-vm` produces the adapter plugin bundle, `neo-riscv-core` provides the local application-engine/provider changes, and `neo-riscv-node` must resolve plugin-facing `Neo` assemblies from the sibling core checkout instead of NuGet whenever that checkout exists. After the build graph is corrected, use the existing StateService plugin plus RPC-based comparison to validate state-root parity block-by-block.

**Tech Stack:** Rust/Cargo, .NET 10/MSBuild, MSTest, Neo CLI, Neo node plugins, LevelDB/StateService, bash validation scripts.

### Task 1: Audit and fix node-to-core references

**Files:**
- Modify: `/home/neo/git/neo-riscv-node/plugins/Directory.Build.props`
- Modify: `/home/neo/git/neo-riscv-node/src/Neo.CLI/Neo.CLI.csproj`

**Step 1: Reproduce the dependency problem**

Run: `cd /home/neo/git/neo-riscv-node && dotnet list plugins/StateService/StateService.csproj package`
Expected: `Neo 3.9.1` appears as a top-level NuGet dependency.

**Step 2: Replace package-based plugin wiring with local-core-first wiring**

Implement a sibling-repo-aware MSBuild rule:
- If `/home/neo/git/neo-riscv-core/src/Neo/Neo.csproj` exists relative to the node repo, use a `ProjectReference`.
- Otherwise fall back to the published `Neo` package so the repo still builds standalone.
- Keep `Neo.CLI` aligned with the same local-core path logic instead of a one-off hardcoded reference.

**Step 3: Verify the fix**

Run:
- `cd /home/neo/git/neo-riscv-node && dotnet list plugins/StateService/StateService.csproj package`
- `cd /home/neo/git/neo-riscv-node && dotnet build neo-node.sln -c Debug`

Expected:
- `Neo` no longer appears as a top-level package for plugin projects when the sibling core repo exists.
- Solution build succeeds.

### Task 2: Re-run the local multi-repo verification matrix

**Files:**
- Use existing verification entrypoints only unless a concrete failure requires edits.

**Step 1: Validate VM repo**

Run:
- `cd /home/neo/git/neo-riscv-vm && cargo fmt --all --check`
- `cd /home/neo/git/neo-riscv-vm && cargo clippy --workspace --all-targets -- -D warnings`
- `cd /home/neo/git/neo-riscv-vm && scripts/verify-all.sh`

Expected: clean format/lint/test status, with only intentionally skipped slow suites.

**Step 2: Validate cross-repo test integration**

Run: `cd /home/neo/git/neo-riscv-vm && scripts/cross-repo-test.sh`
Expected: VM, core, and node test matrices pass with the staged adapter bundle and local core references.

**Step 3: Investigate and fix any failures**

For each failure:
- reproduce with the narrowest project/test filter,
- add or reuse the smallest failing test that isolates the issue,
- patch only the owning repo/files,
- rerun the narrow test before rerunning the broad matrix.

### Task 3: Prepare deterministic state-root comparison deployment

**Files:**
- Modify if needed: `/home/neo/git/neo-riscv-vm/scripts/run-mainnet-stateroot-validation.sh`
- Modify if needed: `/home/neo/git/neo-riscv-vm/scripts/validate-stateroot.sh`
- Inspect/deploy: `/home/neo/git/neo-riscv-node/Plugins`

**Step 1: Build the deployable bundle**

Run: `cd /home/neo/git/neo-riscv-vm && scripts/run-mainnet-stateroot-validation.sh --build-only`
Expected:
- release host library built,
- adapter plugin copied into deploy `Plugins/Neo.Riscv.Adapter`,
- node plugins built from local sources,
- publish output uses the local core fork.

**Step 2: Sanity-check deployment contents**

Verify:
- `Neo.CLI.dll` exists in the deploy directory,
- `Plugins/Neo.Riscv.Adapter/Neo.Riscv.Adapter.dll` exists,
- `Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so` exists on Linux,
- `Plugins/StateService/StateService.dll` and `Plugins/RpcServer/RpcServer.dll` exist.

### Task 4: Run full node parity validation

**Files:**
- Use generated deploy directory under `/home/neo/git/neo-riscv-vm/mainnet-validation` or successor directory.

**Step 1: Launch the RISC-V node**

Run: `cd /home/neo/git/neo-riscv-vm && scripts/run-mainnet-stateroot-validation.sh`
Expected:
- local RPC on `127.0.0.1:10332`,
- StateService enabled,
- node begins syncing.

**Step 2: Compare state roots block-by-block**

Use the monitor loop in `run-mainnet-stateroot-validation.sh` or an improved equivalent to compare:
- local `getstateroot(block)` from the RISC-V node,
- reference `getstateroot(block)` from the baseline Neo node/RPC.

Requirements:
- log every mismatch with exact block index and both root hashes,
- persist progress checkpoints so long validation runs can resume,
- treat missing state roots near the tip as retriable, not silent success.

**Step 3: Define the completion condition**

The task is only complete when either:
- all synced blocks have matching state roots with zero mismatches, or
- a first mismatch is isolated with the exact block index and a reproducible minimal follow-up path.

### Task 5: Report exact compatibility status

**Files:**
- Update or create report only after evidence exists.

**Step 1: Summarize verified status**

Report:
- whether node/plugin builds use local core references,
- which repo-wide validation commands passed,
- highest block compared,
- mismatch count,
- whether validation covered the full intended chain height or stopped earlier.

**Step 2: If a mismatch exists**

Capture:
- first mismatching block,
- local root,
- reference root,
- relevant logs/artifacts,
- likely owning subsystem (VM execution, provider wiring, state service, storage, or deploy mismatch).
