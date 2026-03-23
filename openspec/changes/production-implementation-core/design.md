## Context

Current state: 258 Rust tests passing, 2 C# compat tests passing, but many are placeholder implementations. Previous changes created infrastructure (test files, benchmark harness, profiling hooks) but didn't implement actual validation logic.

Constraints:

- Must maintain 100% backward compatibility with existing NeoVM behavior
- Performance target: within 2x of native NeoVM execution time
- Zero-downtime deployment requirement for production systems

## Goals / Non-Goals

**Goals:**

- Replace all placeholder tests with real NeoVM script execution
- Establish performance baselines with Criterion benchmarks
- Add production-grade observability (metrics, logging, health checks)
- Document all public APIs with rustdoc and C# XML comments
- Create operational runbooks for deployment and troubleshooting

**Non-Goals:**

- Performance optimization beyond 2x threshold (future work)
- Additional opcode implementations (scope: validation only)
- UI/dashboard for monitoring (use existing Prometheus/Grafana)
- Automated deployment pipelines (manual deployment acceptable)

## Decisions

### Decision 1: Real Script Execution vs Unit Mocks

**Choice:** Execute actual NeoVM bytecode scripts in tests
**Rationale:** Placeholder tests don't validate real behavior. Need end-to-end validation.
**Alternative considered:** Mock-based unit tests → rejected, insufficient coverage

### Decision 2: Criterion for Benchmarks

**Choice:** Use Criterion.rs with statistical analysis
**Rationale:** Already added as dependency, industry standard, provides regression detection
**Alternative considered:** Custom timing harness → rejected, reinventing wheel

### Decision 3: Prometheus + tracing for Observability

**Choice:** prometheus crate for metrics, tracing crate for structured logs
**Rationale:** Standard Rust ecosystem tools, widely supported
**Alternative considered:** Custom metrics → rejected, poor ecosystem integration

### Decision 4: Inline rustdoc vs Separate Docs

**Choice:** Inline rustdoc comments in source files
**Rationale:** Co-located with code, enforced by CI, easier to maintain
**Alternative considered:** Separate markdown docs → rejected, drift risk

## Risks / Trade-offs

**Risk:** Real script execution tests may be slow
→ **Mitigation:** Run in parallel, use `cargo nextest` for faster execution

**Risk:** Benchmark baselines may vary across hardware
→ **Mitigation:** Document reference hardware, use relative thresholds (2x native)

**Risk:** Prometheus metrics add runtime overhead
→ **Mitigation:** Use lock-free atomic counters, minimal allocation

**Trade-off:** Comprehensive tests increase CI time
→ **Accepted:** Quality over speed, parallelize where possible
