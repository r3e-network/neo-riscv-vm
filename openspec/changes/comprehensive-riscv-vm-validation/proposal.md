## Why

The current RISC-V VM needs to be efficient, well-designed, correctly and completely implemented to ensure 100% compatibility with NeoVM that runs upon it. Comprehensive cross-repo validation with `neo-riscv-core` and `neo-riscv-node` is required to guarantee production readiness, correct execution semantics, and parity with existing NeoVM behavior.

## What Changes

- Complete implementation of all RISC-V VM instructions to achieve 100% NeoVM compatibility.
- Addition of comprehensive test suites to validate execution correctness and performance.
- Integration of cross-repo tests using `~/git/neo-riscv-core` and `~/git/neo-riscv-node` to ensure system-wide stability.
- Architectural and performance optimizations for efficiency.

## Capabilities

### New Capabilities
- `cross-repo-validation`: Establish cross-repository testing frameworks integrating the VM with `neo-riscv-core` and `neo-riscv-node`.
- `neovm-parity-verification`: Ensure and validate 100% semantic and behavioral parity between the RISC-V VM and NeoVM.
- `vm-efficiency-optimization`: Design and implement performance optimizations for the RISC-V VM.

### Modified Capabilities
- `riscv-instruction-set`: Complete the implementation of the RISC-V instruction set to support all required NeoVM operations.

## Impact

- `neo-riscv-vm` repository (core VM implementation, tests, and adapter).
- Cross-repository testing pipelines in `neo-riscv-core` and `neo-riscv-node`.
- Potential updates to adapter plugins and integration layers to support full compatibility.

## Implementation Status Note (2026-03-24)

The canonical validation artifact is now `scripts/cross-repo-test.sh`, which packages the adapter plugin and validates the committed `neo-riscv-vm`, `neo-riscv-core`, and `neo-riscv-node` workspaces together.
