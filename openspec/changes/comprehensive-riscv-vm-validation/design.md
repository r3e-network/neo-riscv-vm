## Context

The Neo network relies heavily on the correct execution of smart contracts within the NeoVM. As part of integrating a RISC-V VM as an execution engine, it is critical that this VM runs efficiently, is well-designed, and correctly and completely implements 100% of the functionality required for NeoVM parity. This requires a robust validation strategy that spans multiple repositories, specifically `neo-riscv-core` and `neo-riscv-node`, to guarantee production readiness and accurate execution semantics.

## Goals / Non-Goals

**Goals:**
- Design a comprehensive cross-repository testing framework for the RISC-V VM.
- Establish processes and tools for 100% NeoVM compatibility validation.
- Identify and document architectural and performance optimization patterns for the VM.
- Guarantee correctness and semantic parity across all implemented instructions.

**Non-Goals:**
- Refactoring the entire `neo-riscv-core` or `neo-riscv-node` codebases beyond integration points.
- Modifying the core Neo protocol semantics; the goal is to conform to existing NeoVM behavior, not alter it.

## Decisions

- **Cross-Repo Test Orchestration:** Use shell scripts and GitHub Actions (or equivalent CI/CD) to orchestrate cross-repository testing. This ensures that changes in the VM are immediately tested against real-world integration points in the core and node repositories.
- **Parity Verification Strategy:** Implement a differential testing approach where the same smart contract logic is executed on both the reference NeoVM and the new RISC-V VM, asserting identical state transitions, gas consumption, and return values.
- **Optimization Strategy:** Profiling will drive optimizations. We will utilize benchmark tools to identify hotspots in instruction execution and state access, and optimize these areas iteratively. PolkaVM specific optimizations will be explored where applicable to the architecture.

## Risks / Trade-offs

- [Risk: Differential Testing Complexity] Ensuring identical state environments for both VMs can be challenging.
  - Mitigation: Develop a robust, isolated state-fixture and snapshotting mechanism shared across tests.
- [Risk: Cross-Repo CI Flakiness] Integration tests spanning multiple repos can be brittle due to synchronization issues.
  - Mitigation: Pin repository versions or commit hashes during test runs and implement retry mechanisms for transient failures.
- [Risk: Performance vs. Correctness] Aggressive optimizations might introduce subtle semantic deviations.
  - Mitigation: The parity verification suite must run strictly on all optimized builds before they are considered valid.

## Implementation Status Note (2026-03-24)

- Cross-repo validation is now exercised through `scripts/cross-repo-test.sh`.
- The validated workspace includes targeted integration changes in both `neo-riscv-core` and `neo-riscv-node`.
- Current validation proves workspace correctness, but not an upstream-zero-diff packaging story.
