## 1. Setup Validation Framework

- [x] 1.1 Scaffold the differential testing environment in `neo-riscv-vm`
- [x] 1.2 Implement the cross-repo CI script referencing `neo-riscv-core` and `neo-riscv-node`
- [x] 1.3 Configure repository version pinning logic in CI
- [x] 1.4 Validate initial test run executes properly (even if tests fail)

## 2. RISC-V Instruction Completion

- [x] 2.1 Audit missing or partially implemented RISC-V instructions compared to NeoVM requirements
- [x] 2.2 Implement remaining instructions in the VM core
- [x] 2.3 Write isolated unit tests for newly implemented instructions handling edge cases
- [x] 2.4 Run regression tests to verify no existing functionality is broken

## 3. Parity Validation Setup

- [x] 3.1 Integrate state mocking and snapshotting for differential testing
- [x] 3.2 Implement state equality assertions (storage, return values, events)
- [x] 3.3 Implement gas consumption parity checking or establish bounds
- [x] 3.4 Execute comprehensive test suite with NeoVM baseline and investigate failures

## 4. Performance Optimization

- [x] 4.1 Setup performance profiling in the test suite
- [x] 4.2 Establish baseline benchmark results
- [x] 4.3 Identify and optimize state access bottlenecks
- [x] 4.4 Profile instruction execution hotspots and optimize
- [x] 4.5 Run regression benchmarking to validate optimizations do not degrade performance