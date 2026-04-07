# Copy-on-Write Alias Propagation Design

## Current Implementation Analysis

### Bottleneck Identification

**Problem**: `propagate_update` performs deep clones for all aliases, even when not modified.

Current flow:

```
Modify Array[0] → propagate_update → replace_alias → clone() for EVERY alias
```

**Cost Analysis**:

- `replace_alias` calls `clone()` on every matching `compound_id`
- `StackValue::clone()` deep-copies entire Vec for Array/Struct/Map
- 20 call sites (APPEND/SETITEM/REMOVE/CLEAR/POPITEM/REVERSE/MEMCPY)
- Cost scales with: alias count × compound size

### Current Code Structure

**StackValue** (runtime_types.rs:5-19):

```rust
pub(crate) enum StackValue {
    Array(u64, Vec<StackValue>),      // Direct Vec ownership
    Struct(u64, Vec<StackValue>),     // Direct Vec ownership
    Map(u64, Vec<(StackValue, StackValue)>), // Direct Vec ownership
    Buffer(u64, Vec<u8>),             // Direct Vec ownership
    // ... primitives
}
```

**propagate_update** (runtime_types.rs:212-241):

- Iterates stack/locals/static_fields
- Calls `replace_alias` for each value
- No sharing, only cloning

**replace_alias** (runtime_types.rs:243-271):

- Matches `compound_id`
- Executes `*target = updated.clone()` (deep copy)
- Recursively processes nested compounds

## Proposed Solution: Rc<RefCell<Vec>>

### Design Rationale

**Why Rc<RefCell>**:

1. **Reference counting**: Tracks alias count automatically
2. **Interior mutability**: Allows mutation through shared reference
3. **CoW detection**: `Rc::strong_count() > 1` indicates sharing
4. **Zero-cost reads**: Direct dereference, no runtime overhead
5. **Rust idiomatic**: Follows standard library patterns

**Alternatives Rejected**:

- Manual refcount in HashMap: Global state complexity, lookup overhead
- Arc<Mutex>: Unnecessary thread-safety overhead (single-threaded guest)
- Generational indices: Complex lifetime management

### New StackValue Definition

```rust
use std::rc::Rc;
use std::cell::RefCell;

pub(crate) enum StackValue {
    // Immutable types: unchanged
    Integer(i64),
    BigInteger(Vec<u8>),
    ByteString(Vec<u8>),
    Boolean(bool),
    Pointer(usize),
    Null,
    Interop(u64),
    Iterator(u64),

    // Compound types: Rc-wrapped for CoW
    Array(u64, Rc<RefCell<Vec<StackValue>>>),
    Struct(u64, Rc<RefCell<Vec<StackValue>>>),
    Map(u64, Rc<RefCell<Vec<(StackValue, StackValue)>>>),
    Buffer(u64, Rc<RefCell<Vec<u8>>>),
}
```

**Memory overhead**:

- Before: `Vec<T>` = 24 bytes (ptr + len + cap)
- After: `Rc<RefCell<Vec<T>>>` = 8 bytes (Rc ptr) + heap(16 bytes refcount + 24 bytes Vec) = 8 + 40 = 48 bytes
- Overhead: 24 bytes per compound value
- **Trade-off**: 24 bytes overhead vs avoiding N deep clones (N = alias count)

## Implementation Strategy

### Phase 1: Modify StackValue Definition

**File**: `crates/neo-riscv-guest/src/runtime_types.rs`

**Changes**:

1. Add imports: `use std::rc::Rc; use std::cell::RefCell;`
2. Wrap compound types in `Rc<RefCell<T>>`
3. Update `Clone` implementation to use `Rc::clone` (cheap refcount bump)
4. Update `PartialEq` to compare inner values

**Impact**: All construction and access code must be updated.

### Phase 2: Refactor propagate_update

**Old behavior**:

```rust
fn replace_alias(target: &mut StackValue, updated: &StackValue) {
    if compound_id(target) == compound_id(updated) {
        *target = updated.clone();  // Deep copy entire Vec
    }
}
```

**New behavior**:

```rust
fn replace_alias(target: &mut StackValue, updated: &StackValue) {
    if compound_id(target) == compound_id(updated) {
        *target = updated.clone();  // Rc::clone, only bumps refcount
    }
}
```

**Result**: `propagate_update` becomes reference sharing, not data copying.

### Phase 3: Add CoW to Mutation Operations

**Pattern for all 20 call sites**:

```rust
// Before mutation
let items = match &array_value {
    StackValue::Array(id, items) => {
        if Rc::strong_count(items) > 1 {
            // Shared: clone data
            Rc::new(RefCell::new((*items.borrow()).clone()))
        } else {
            // Exclusive: reuse
            Rc::clone(items)
        }
    }
    _ => return Err("not an array"),
};

// Mutate
items.borrow_mut().push(value);

// Create updated value
let updated = StackValue::Array(id, items);

// Propagate (now cheap: only Rc::clone)
propagate_update(&updated, stack, locals, static_fields, affected);
```

**Call sites to update** (lib.rs):

- APPEND (line ~1078, ~1090)
- SETITEM (line ~1203, ~1219, ~1239)
- REMOVE (line ~1262, ~1278, ~1295)
- CLEAR (line ~1311, ~1321, ~1331, ~1341)
- POPITEM (line ~1361, ~1377, ~1393, ~1410)
- REVERSE (line ~1441, ~1453, ~1465)
- MEMCPY (line ~1641)

### Phase 4: Update Helper Functions

**Files to update**:

- `runtime_types.rs`: All StackValue construction/access
- `helpers.rs`: `item_to_*` conversion functions
- `lib.rs`: Opcode implementations reading compound types

**Pattern**:

```rust
// Old: direct access
let items = &array.1;

// New: borrow through RefCell
let items = array.1.borrow();
```

## Verification Strategy

### Test Coverage

**Existing tests** (must all pass):

- 258 Rust tests (codec + callback + neovm_json + interop + host)
- 2 C# compat tests (RiscVVmTests + RiscVVmCompatTests)

**Alias semantics validation**:

```rust
// Test: Modify one alias, others see change
let arr = Array([1, 2, 3]);
let alias1 = arr;
let alias2 = arr;
arr[0] = 99;
assert_eq!(alias1[0], 99);  // Must see change
assert_eq!(alias2[0], 99);  // Must see change

// Test: CoW triggers on modification
let arr = Array([1, 2, 3]);
let alias = arr;
assert_eq!(Rc::strong_count(&arr.1), 2);  // Shared
arr[0] = 99;  // Triggers CoW
assert_eq!(Rc::strong_count(&arr.1), 1);  // Now exclusive
```

### Performance Benchmarks

**Criterion benchmarks** (to be added):

```rust
// Baseline: current implementation
bench_alias_propagation_baseline();

// CoW: new implementation
bench_alias_propagation_cow();

// Expected: 10-18% improvement
```

**Scenarios**:

1. Single alias (no benefit, slight overhead)
2. 5 aliases (moderate benefit)
3. 10+ aliases (high benefit)
4. Large compound types (1000+ elements)

## Risk Assessment

### Correctness Risks

**Risk 1: RefCell borrow violations**

- **Mitigation**: Single-threaded execution, sequential access pattern
- **Detection**: Runtime panic if violated

**Risk 2: Alias semantics broken**

- **Mitigation**: Comprehensive test coverage
- **Detection**: Existing 258 tests + new alias tests

**Risk 3: Memory leaks from reference cycles**

- **Mitigation**: StackValue doesn't form cycles (tree structure)
- **Detection**: Valgrind/MIRI checks

### Performance Risks

**Risk 1: Refcount overhead > clone savings**

- **Scenario**: Single alias, small compounds
- **Mitigation**: Benchmark validates net benefit
- **Fallback**: Revert if <5% improvement

**Risk 2: RefCell borrow overhead**

- **Impact**: ~1-2 CPU cycles per borrow
- **Mitigation**: Negligible vs clone cost (100s-1000s cycles)

**Risk 3: Memory overhead (24 bytes/compound)**

- **Impact**: Acceptable for performance gain
- **Mitigation**: Monitor memory usage in benchmarks

## Expected Outcomes

**Performance**:

- 10-18% improvement in alias-heavy workloads
- Neutral or slight regression (<2%) in single-alias cases
- Net positive across typical Neo contract patterns

**Code quality**:

- Cleaner separation: sharing vs mutation
- More Rust-idiomatic (Rc/RefCell standard pattern)
- Slightly increased complexity (RefCell borrows)

**Maintenance**:

- Future optimizations easier (e.g., Rc → Arc for parallelism)
- Clear CoW semantics for contributors

## Rollback Plan

If performance targets not met:

1. Keep design doc for future reference
2. Revert StackValue to direct Vec ownership
3. Investigate alternative optimizations (e.g., arena allocation)

## Next Steps

1. **Approval**: Team lead review this design
2. **Implementation**: Phase 1-4 execution
3. **Validation**: Run all 260 tests
4. **Benchmarking**: Criterion performance validation
5. **Integration**: Merge if 10%+ improvement confirmed
