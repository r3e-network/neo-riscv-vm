## Context

The Neo RISC-V VM has achieved functional completeness with all tests passing (258 Rust + 2 C# compatibility). The system consists of:

- 4-crate Rust workspace (ABI, guest interpreter, guest-module, host runtime)
- C# adapter plugin (Neo.Riscv.Adapter) bridging to Neo N3
- Three-repo architecture: neo-riscv-vm → neo-riscv-core → neo-riscv-node

Current state: Production-hardened with FFI safety, TryStack fix, all opcode corrections applied. However, systematic validation of performance, complete compatibility, and cross-repo integration is needed before production deployment.

**Constraints:**

- Must maintain 100% backward compatibility with existing NeoVM contracts
- Cannot break PolkaVM v0.32.0 runtime assumptions
- Must work within Neo N3 gas pricing model
- Cross-repo testing requires coordinated builds across three repositories

## Goals / Non-Goals

**Goals:**

- Establish performance baseline and identify optimization opportunities
- Validate 100% NeoVM opcode compatibility with comprehensive edge case coverage
- Verify cross-repository integration works correctly in realistic scenarios
- Document performance characteristics for production capacity planning
- Identify and fix any remaining architectural inefficiencies

**Non-Goals:**

- Rewriting core PolkaVM runtime (use as-is)
- Changing Neo N3 consensus protocol
- Implementing new NeoVM opcodes beyond current spec
- Performance optimization beyond 2x of native NeoVM (diminishing returns)

## Decisions

### Decision 1: Benchmark Framework - Criterion.rs + Custom Harness

**Choice:** Use Criterion.rs for micro-benchmarks, custom harness for macro-benchmarks

**Rationale:**

- Criterion provides statistical rigor for Rust-level performance
- Custom harness needed for cross-language (Rust↔C#) and cross-repo scenarios
- Allows comparison against native NeoVM execution in neo-riscv-core

**Alternatives Considered:**

- Pure Criterion: Cannot handle C# integration or cross-repo scenarios
- BenchmarkDotNet only: Misses Rust-level bottlenecks in guest interpreter

### Decision 2: Compatibility Test Strategy - Opcode Matrix + Fuzzing

**Choice:** Exhaustive opcode matrix (256 opcodes × edge cases) + property-based fuzzing

**Rationale:**

- Matrix ensures systematic coverage of all opcodes and error paths
- Fuzzing discovers unexpected interactions between opcodes
- Validates against NeoVM reference implementation behavior

**Alternatives Considered:**

- Manual test cases only: Insufficient coverage, misses edge cases
- Fuzzing only: No guarantee of systematic opcode coverage

### Decision 3: Cross-Repo Test Orchestration - Bash Scripts + CI Matrix

**Choice:** Bash orchestration scripts with dependency-aware build order

**Rationale:**

- Simple, transparent, works in CI and local environments
- Handles three-repo dependency chain: vm → core → node
- Can run subset tests (vm-only, vm+core, full-stack)

**Alternatives Considered:**

- Monorepo: Rejected due to separate ownership and release cycles
- Docker Compose: Overkill for build orchestration, adds complexity

### Decision 4: Performance Profiling - perf + Flamegraph + Custom Gas Tracking

**Choice:** Linux perf for CPU profiling, custom instrumentation for gas accuracy

**Rationale:**

- perf provides low-overhead CPU profiling for hotspot identification
- Custom gas tracking validates pricing.rs against NeoVM reference
- Flamegraphs visualize call stacks for optimization targeting

**Alternatives Considered:**

- Valgrind: Too slow for realistic workload profiling
- Built-in Rust profiling only: Misses C# FFI boundary overhead

## Risks / Trade-offs

**Risk:** Performance benchmarks may reveal fundamental PolkaVM overhead that cannot be optimized
→ **Mitigation:** Establish acceptable performance threshold (within 2x of native NeoVM). Document trade-offs of safety and isolation.

**Risk:** Cross-repo tests may be flaky due to timing or environment differences
→ **Mitigation:** Use deterministic test scenarios, fixed gas limits, isolated test environments. Retry logic for transient failures.

**Risk:** Comprehensive compatibility testing may uncover edge cases requiring guest blob regeneration
→ **Mitigation:** Automate guest blob regeneration in CI. Version guest blobs to track changes.

**Risk:** Optimization changes may introduce subtle bugs in production-hardened code
→ **Mitigation:** All optimizations must pass full test suite (258 Rust + 2 C# + new validation tests). Use feature flags for risky changes.

**Trade-off:** Exhaustive testing increases CI time significantly
→ **Acceptance:** Validation quality is more important than CI speed for production readiness. Use parallel test execution and caching.

## Migration Plan

**Phase 1: Baseline Establishment (No Code Changes)**

1. Add benchmark suite to neo-riscv-vm
2. Run baseline performance measurements
3. Document current performance characteristics

**Phase 2: Validation Infrastructure**

1. Add compatibility test matrix to neo-riscv-vm
2. Add cross-repo test orchestration scripts
3. Integrate into CI pipelines

**Phase 3: Optimization (If Needed)**

1. Apply targeted optimizations based on profiling data
2. Validate each optimization maintains test pass rate
3. Regenerate guest blob if guest interpreter changes

**Phase 4: Documentation**

1. Document performance characteristics
2. Create optimization guidelines for future development
3. Update architecture documentation with validation findings

**Rollback Strategy:**

- All changes are additive (tests, benchmarks, scripts)
- No breaking changes to existing APIs
- If optimization introduces bugs, revert specific optimization commit while keeping validation infrastructure

## Open Questions

1. **Performance Threshold:** What is acceptable performance relative to native NeoVM? (Proposed: within 2x)
2. **CI Resource Limits:** How much CI time budget for comprehensive cross-repo testing? (Proposed: 60min max)
3. **Fuzzing Duration:** How long should property-based fuzzing run? (Proposed: 10min per CI run, extended runs nightly)
4. **Optimization Priority:** Which bottlenecks should be optimized first if multiple are found? (Proposed: prioritize by impact × frequency)
