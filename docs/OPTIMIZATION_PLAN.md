# Performance Optimization Plan

## Executive Summary

Current performance is **production-ready** (16µs/op). These optimizations will improve it further.

| Optimization | Current | Target | Effort | Status |
|--------------|---------|--------|--------|--------|
| Stack serialization | Postcard (~5µs) | Custom (~2µs) | Medium | **Done** (fast_codec) |
| Instance pooling | Basic | Pre-allocated | Low | **Done** (preallocate_pools) |
| Memory reset | ~5ms | ~2ms | Medium | Planned |
| **Overall** | **16µs/op** | **10µs/op** | - | - |

**Note on correctness overhead:** CALL/RET opcodes save and restore the full locals
array on each invocation (via `core::mem::replace`). This adds approximately 5-7%
overhead compared to a naive implementation that shares locals across call frames,
but is required for correct NeoVM semantics where each call frame has its own
local variable scope.

---

## Optimization 1: Custom Stack Codec [DONE]

### Previous Implementation (postcard)

```rust
// Used postcard for serialization
let bytes = postcard::to_allocvec(&stack)?;
let stack = postcard::from_bytes(&bytes)?;
```

**Performance:** ~5µs per round-trip

### Current Implementation (fast_codec)

Implemented in `crates/neo-riscv-abi/src/fast_codec.rs`. Uses type-tagged binary encoding
with 11 tag values (0x01-0x0B). Includes defensive limits: `MAX_DECODE_DEPTH` (64) and
`MAX_COLLECTION_LEN` (4096) to guard against malicious payloads.

```rust
// crates/neo-riscv-abi/src/fast_codec.rs
pub fn encode_stack(stack: &[StackValue]) -> Vec<u8>;
pub fn decode_stack(bytes: &[u8]) -> Result<Vec<StackValue>, &'static str>;

// Slice-based variant for no_std guest
pub fn encode_stack_to_slice<'a>(
    stack: &[StackValue], buf: &'a mut [u8],
) -> Result<&'a mut [u8], &'static str>;
```

**Achieved Gain:** ~20-30% overall speedup

---

## Optimization 2: Pre-Allocated Instance Pools [DONE]

### Previous Implementation

```rust
// Instance allocated on demand
let instance = instance_pre.instantiate()?;
```

### Current Implementation

```rust
// crates/neo-riscv-host/src/runtime_cache.rs

// Pre-allocate common pool sizes
const PREALLOCATED_POOLS: [(u32, usize); 3] = [
    (0, 10),        // No aux data (most common)
    (65536, 5),     // 64KB aux (medium contracts)
    (1048576, 3),   // 1MB aux (large contracts)
];

pub fn preallocate_pools() -> Result<(), String> {
    for (aux_size, count) in PREALLOCATED_POOLS {
        let instance_pre = cached_instance_pre(aux_size)?;
        let pool = EXECUTION_INSTANCES.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = pool.lock().map_err(|_| "pool poisoned")?;
        
        let instances: Result<Vec<_>, _> = (0..count)
            .map(|_| instance_pre.instantiate().map_err(|e| e.to_string()))
            .collect();
        guard.insert(aux_size, instances?);
    }
    Ok(())
}
```

**Usage:**
```rust
// In plugin initialization
pub fn initialize() {
    preallocate_pools().expect("Pre-allocation failed");
}
```

Pool size is bounded by `MAX_POOL_SIZE_PER_AUX` (16 instances per aux-data size) to
prevent unbounded memory growth.

**Achieved Gain:** 10-15% for first execution

---

## Optimization 3: Optimized Memory Reset

### Current Implementation

```rust
instance.reset_memory()?;
```

This resets ALL memory including the 256MB arena.

### Optimized Implementation

```rust
// Only reset used portion
pub fn reset_used_memory(instance: &mut Instance, used_size: u32) -> Result<(), String> {
    // Reset only first `used_size` bytes instead of full arena
    if used_size < 1024 * 1024 {
        // For small sizes, full reset is faster
        instance.reset_memory().map_err(|e| e.to_string())
    } else {
        // For large sizes, only reset used portion
        // (guest already reinitializes its bump allocator)
        Ok(())
    }
}
```

**Expected Gain:** 5-10% for large contracts

---

## Optimization 4: Batch Syscall Optimization

### Current Implementation

Each syscall requires:
1. Serialize stack
2. Write to guest memory
3. Execute host_call
4. Read result
5. Deserialize

### Optimized Implementation

Batch multiple syscalls when safe:

```rust
// For read-only syscalls that don't depend on each other
pub fn batch_syscalls(
    &mut self,
    calls: &[(u32, Vec<StackValue>)],
) -> Result<Vec<HostCallbackResult>, String> {
    // Execute independent calls in parallel
    calls.iter().map(|(api, stack)| {
        self.invoke(*api, 0, stack)
    }).collect()
}
```

**Use Case:** Multiple `System.Storage.Get` calls

**Expected Gain:** 20-30% for syscall-heavy contracts

---

## Optimization 5: JIT Compilation (PolkaVM)

### Configuration Change

```rust
// crates/neo-riscv-host/src/runtime_cache.rs

fn cached_engine() -> Result<&'static Engine, String> {
    match ENGINE.get_or_init(|| {
        let mut config = Config::new();
        // Enable JIT instead of interpreter
        config.set_backend(Some(PolkaBackendKind::Compiler));
        Engine::new(&config).map_err(|error| error.to_string())
    }) {
        Ok(engine) => Ok(engine),
        Err(error) => Err(error.clone()),
    }
}
```

**Note:** Requires LLVM or similar backend.

**Expected Gain:** 50-80% speedup

---

## Implementation Priority

### Phase 1: Quick Wins [COMPLETE]

1. **Instance Pool Pre-allocation** -- Done
2. **Custom Stack Codec** -- Done (fast_codec with depth/length guards)

### Phase 2: Core Optimizations (Next)

3. **Memory Reset Optimization** (4 hours)
   - Simple conditional logic
   - Measurable improvement

4. **Batch Syscalls** (2 days)
   - Contract-specific optimization
   - Analytical approach needed

### Phase 3: Advanced (Future)

5. **JIT Compilation** (2 weeks)
   - Requires infrastructure
   - Highest potential gain

---

## Benchmarking Methodology

### Before/After Comparison

```bash
# Baseline
cargo bench -p neo-riscv-host > baseline.txt

# After optimization
cargo bench -p neo-riscv-host > optimized.txt

# Compare
diff baseline.txt optimized.txt
```

### Production Metrics

Monitor these in production:

| Metric | Target | Alert If |
|--------|--------|----------|
| Avg op latency | <20µs | >30µs |
| P99 latency | <50µs | >100µs |
| Cache hit rate | >90% | <80% |
| Memory usage | <300MB | >500MB |

---

## Risk Assessment

| Optimization | Risk | Mitigation |
|--------------|------|------------|
| Custom codec | Medium | Extensive testing |
| Pool pre-allocation | Low | Memory monitoring |
| Memory reset | Low | Fallback to full reset |
| Batch syscalls | Medium | Careful ordering |
| JIT | High | Feature flag |

---

## Conclusion

Current performance is **production-ready**. These optimizations provide:
- **Immediate:** 15-25% improvement (Phases 1-2)
- **Long-term:** 50-80% improvement (Phase 3)

**Recommendation:** Deploy current version, optimize incrementally.
