# Neo RISC-V VM Architecture

**Version:** 1.0  
**Status:** Production Ready  
**Last Updated:** 2026-03-26

---

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [System Architecture](#system-architecture)
4. [Component Details](#component-details)
5. [Data Flow](#data-flow)
6. [Memory Management](#memory-management)
7. [Security Model](#security-model)
8. [Performance Characteristics](#performance-characteristics)
9. [Integration Points](#integration-points)
10. [Error Handling](#error-handling)

---

## Overview

The Neo RISC-V VM is a plugin-first execution stack for Neo N3. It uses:

- **RISC-V** as the primary virtual machine (via PolkaVM)
- **Internal NeoVM compatibility layer** for perfect backward compatibility
- **External adapter ownership** so core no longer carries an in-tree RISC-V bridge implementation

### Key Metrics

| Metric | Value |
|--------|-------|
| VM workspace tests | 376 |
| Cross-repo core+node tests | 2022 |
| Compatibility | 100% NeoVM compatible |
| Performance | ~16µs per operation |
| Memory Overhead | 256MB guest arena |
| Integration model | Workspace-scoped adapter/plugin |

---

## Design Principles

### 1. Preserve Contract Semantics

Preserve:
- user contract behavior
- native contract behavior
- syscall behavior
- gas accounting semantics

### 2. Perfect Compatibility

All NeoVM bytecode executes identically:
- Same opcodes
- Same gas costs
- Same error messages
- Same stack behavior

### 3. Security First

- Sandboxed execution
- Memory isolation
- Resource limits
- Validated syscalls

### 4. Performance Acceptable

- Sub-50µs per operation
- Optimized critical paths
- Instance pooling
- Efficient serialization

---

## System Architecture

### High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER LAYER                                     │
│  Smart Contracts (NEP-17, NFT, DeFi) - unchanged                           │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           NEO CORE LAYER (C#)                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ApplicationEngine                                                   │   │
│  │ ├── Native contracts (Ledger, NeoToken, GasToken, Policy, etc.)    │   │
│  │ ├── VM execution context                                           │   │
│  │ └── Syscall dispatcher                                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    │ IApplicationEngineProvider            │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ RiscvApplicationEngineProvider                                      │   │
│  │ └── Auto-registered via plugin                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ P/Invoke
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FFI LAYER (C/Rust)                               │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ libneo_riscv_host.so                                                │   │
│  │ ├── neo_riscv_execute_script_with_host()                           │   │
│  │ ├── Host callback registration                                     │   │
│  │ └── Result marshaling                                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Rust FFI
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         HOST RUNTIME LAYER (Rust)                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Runtime Cache                                                       │   │
│  │ ├── Engine (PolkaVM singleton)                                     │   │
│  │ ├── Module cache (aux_size → Module)                               │   │
│  │ ├── InstancePre cache (aux_size → InstancePre)                     │   │
│  │ └── Instance pool (aux_size → Vec<Instance>)                       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ Execution Context                                                   │   │
│  │ ├── Script loading                                                  │   │
│  │ ├── Stack serialization                                             │   │
│  │ ├── Gas accounting                                                  │   │
│  │ └── Host callbacks                                                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ Internal syscalls
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         GUEST VM LAYER (RISC-V)                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ guest.polkavm (PolkaVM RISC-V binary)                               │   │
│  │                                                                     │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │ NeoVM Interpreter (no_std Rust)                              │   │   │
│  │  │                                                             │   │   │
│  │  │  ┌─────────────────────────────────────────────────────┐   │   │   │
│  │  │  │ Opcode Handler                                       │   │   │   │
│  │  │  │ ├── Push opcodes (0x00-0x21)                        │   │   │   │
│  │  │  │ ├── Control flow (0x22-0x3F)                        │   │   │   │
│  │  │  │ ├── Stack ops (0x43-0x54)                           │   │   │   │
│  │  │  │ ├── Slot ops (0x56-0x87)                            │   │   │   │
│  │  │  │ ├── Arithmetic (0x99-0xBB)                          │   │   │   │
│  │  │  │ └── Compound types (0xBE-0xD3)                      │   │   │   │
│  │  │  └─────────────────────────────────────────────────────┘   │   │   │
│  │  │                                                             │   │   │
│  │  │  ┌─────────────────────────────────────────────────────┐   │   │   │
│  │  │  │ Stack Management                                     │   │   │   │
│  │  │  │ ├── Evaluation stack (max 2048 items)               │   │   │   │
│  │  │  │ ├── Alt stack                                       │   │   │   │
│  │  │  │ ├── Local slots                                     │   │   │   │
│  │  │  │ └── Static fields                                   │   │   │   │
│  │  │  └─────────────────────────────────────────────────────┘   │   │   │
│  │  │                                                             │   │   │
│  │  │  ┌─────────────────────────────────────────────────────┐   │   │   │
│  │  │  │ Execution Context                                    │   │   │   │
│  │  │  │ ├── Instruction pointer                             │   │   │   │
│  │  │  │ ├── Call stack (saves/restores locals per frame)     │   │   │   │
│  │  │  │ └── Try frames (TRY/CATCH/FINALLY, end_ip field)    │   │   │   │
│  │  │  └─────────────────────────────────────────────────────┘   │   │   │
│  │  │                                                             │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │ Memory Management                                            │   │   │
│  │  │ ├── Bump allocator (256MB arena)                            │   │   │
│  │  │ ├── Stack buffer (1MB)                                      │   │   │
│  │  │ └── Result buffer (64KB)                                    │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  │  ┌─────────────────────────────────────────────────────────────┐   │   │
│  │  │ Host Interface                                               │   │   │
│  │  │ ├── host_on_instruction() - gas accounting                  │   │   │
│  │  │ └── host_call() - syscalls                                  │   │   │
│  │  └─────────────────────────────────────────────────────────────┘   │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Details

### C# Adapter (`Neo.Riscv.Adapter`)

**Purpose:** Bridge between Neo core and Rust host

**Key Classes:**
- `RiscvAdapterPlugin` - Plugin entry point, resolves and registers provider
- `RiscvApplicationEngine` - ApplicationEngine implementation
- `RiscvApplicationEngineProvider` - IApplicationEngineProvider implementation
- `NativeRiscvVmBridge` - FFI wrapper

**Lifecycle:**
1. Neo loads plugin from `Plugins/` directory
2. Plugin constructor resolves the provider and sets `ApplicationEngine.Provider`
3. All subsequent engine creation goes through the adapter-owned RISC-V path

### Rust Host (`neo-riscv-host`)

**Purpose:** RISC-V runtime and FFI layer

**Key Modules:**
- `lib.rs` - Public API, execution functions
- `runtime_cache.rs` - Instance/module caching
- `bridge.rs` - Host function registration (logs diagnostic errors)
- `pricing.rs` - Gas accounting
- `ffi.rs` - C FFI exports

**Key Features:**
- Instance pooling for reuse
- Module caching by aux_size
- Zero-copy stack serialization where possible

### Rust Guest (`neo-riscv-guest`)

**Purpose:** NeoVM interpreter running inside RISC-V

**Key Modules:**
- `lib.rs` - Interpreter entry points
- `opcodes.rs` - Opcode definitions and handlers
- `runtime_types.rs` - Stack types and operations
- `helpers.rs` - Utility functions

**Key Features:**
- No standard library (no_std)
- Bump allocator for memory
- Fixed-size buffers for host communication

---

## Data Flow

### Contract Execution Flow

```
1. C#: ApplicationEngine.Create()
   ↓
2. C#: RiscvApplicationEngine.Execute()
   ↓
3. FFI: neo_riscv_execute_script_with_host()
   ↓
4. Rust: execute_script_with_context()
   ↓
5. Rust: cached_execution_instance(aux_size)
   ↓
6. Rust: Write script to guest memory
   ↓
7. Rust: instance.call_typed("execute", ...)
   ↓
8. RISC-V: execute_inner() in guest
   ↓
9. RISC-V: Interpret NeoVM opcodes
   ↓
10. RISC-V (if SYSCALL): host_call_import()
       ↓
11. Rust: Invoke C# callback
       ↓
12. C#: Execute native contract
       ↓
13. C#: Return result
       ↓
14. Rust: Encode result
       ↓
15. RISC-V: Continue execution
   ↓
16. RISC-V: encode_result()
   ↓
17. Rust: decode result
   ↓
18. FFI: Return NativeExecutionResult
   ↓
19. C#: Convert to StackItem[]
   ↓
20. C#: Push to ResultStack
```

### Syscall Flow

```
Guest VM                    Host Runtime                    C# Core
    │                            │                              │
    │  1. Execute SYSCALL        │                              │
    │  opcode                    │                              │
    │                            │                              │
    ▼                            │                              │
┌──────────┐                     │                              │
│ Serialize│                     │                              │
│ stack    │                     │                              │
└────┬─────┘                     │                              │
     │  2. Write to aux          │                              │
     │     memory                │                              │
     │                           │                              │
     │──────────────────────────▶│                              │
     │                           │                              │
     │                           │  3. Read stack               │
     │                           │                             │
     │                           │  4. Invoke callback         │
     │                           │                             │
     │                           │────────────────────────────▶│
     │                           │                             │
     │                           │                             │ 5. Execute
     │                           │                             │    syscall
     │                           │                             │
     │                           │◀────────────────────────────│
     │                           │                             │
     │                           │  6. Encode result           │
     │                           │                             │
     │◀──────────────────────────│                             │
     │                           │                             │
     │  7. Read result           │                             │
     │     from aux              │                             │
     │                           │                             │
┌────▼─────┐                     │                             │
│ Deserialize                     │                             │
│ result   │                     │                             │
└──────────┘                     │                             │
```

---

## Memory Management

### Host Memory (Rust)

| Component | Size | Purpose |
|-----------|------|---------|
| Guest blob | ~2MB | Compiled PolkaVM binary |
| Engine | ~1MB | PolkaVM runtime |
| Module cache | Variable | Cached modules per aux_size |
| Instance pool | Variable | Reusable instances |

### Guest Memory (RISC-V)

| Region | Size | Access |
|--------|------|--------|
| Code | ~500KB | Read-only |
| Heap | 256MB | Read-write (bump allocator) |
| Aux Data | Variable | Read-write (script + stack) |
| Stack | 64KB | Read-write (guest stack) |

### Memory Lifecycle

1. **Allocation**: Guest uses bump allocator from 256MB arena
2. **Reset**: Instance.reset_memory() clears all guest memory
3. **Reuse**: Instances returned to pool for next execution
4. **Bounds**: All accesses bounds-checked by PolkaVM

---

## Security Model

### Sandboxing

| Layer | Protection |
|-------|------------|
| PolkaVM | Memory isolation, bounds checking |
| Host | API validation, resource limits |
| C# | Standard .NET security |

### Resource Limits

| Resource | Limit | Enforcement |
|----------|-------|-------------|
| Memory | 256MB | Hard limit via arena size |
| Stack items | 2048 | Checked on each push |
| Call depth | 1024 | Checked on each call |
| Gas | Transaction limit | Checked per opcode |
| Codec decode depth | 64 | MAX_DECODE_DEPTH in fast codec |
| Codec collection len | 4096 | MAX_COLLECTION_LEN in fast codec |
| Result size | 16MB | MAX_RESULT_SIZE on host |
| Instance pool per aux | 16 | MAX_POOL_SIZE_PER_AUX on host |

### Trust Boundaries

```
Untrusted:
  - Guest VM code
  - Smart contract bytecode
  - User input

Trusted:
  - Host runtime
  - C# core
  - Native contracts
```

---

## Performance Characteristics

### Baseline Metrics

| Operation | Time | Notes |
|-----------|------|-------|
| PUSH1 | ~0.5µs | In-guest |
| ADD | ~0.5µs | In-guest |
| SYSCALL | ~5µs | Host round-trip |
| Instance creation | ~50ms | Cold start |
| Instance reuse | ~2ms | From pool |

### Throughput

| Workload | Ops/sec | Notes |
|----------|---------|-------|
| Pure arithmetic | ~60,000 | In-guest |
| Syscall-heavy | ~2,000 | Host-bound |
| Mixed | ~20,000 | Typical |

### Memory Overhead

| Component | Overhead |
|-----------|----------|
| Per node | +256MB (guest arena) |
| Per execution | ~1-10MB (depends on contract) |
| Cumulative | Instances pooled, not leaked |

### Optimization Opportunities

1. **JIT compilation** - Enable PolkaVM JIT (50-80% gain)
2. **Instance pre-allocation** - Reduce cold start (10-15% gain)

*Note:* Custom serialization (fast codec) has already replaced postcard.

See [Optimization Plan](./OPTIMIZATION_PLAN.md) for details.

---

## Integration Points

### C# Integration

```csharp
// Plugin registers the provider on load
public class RiscvAdapterPlugin : Plugin
{
    public RiscvAdapterPlugin()
    {
        ApplicationEngine.Provider = RiscvApplicationEngineProviderResolver.ResolveRequiredProvider();
    }
}

// Provider creates RISC-V engines
public class RiscvApplicationEngineProvider : IApplicationEngineProvider
{
    public ApplicationEngine Create(...)
    {
        return new RiscvApplicationEngine(...);
    }
}
```

### FFI Integration

```rust
// C-exported function
#[no_mangle]
pub extern "C" fn neo_riscv_execute_script_with_host(
    script: *const u8,
    script_len: usize,
    // ... other params
    result: *mut NativeExecutionResult,
) -> bool {
    // Implementation
}
```

### Guest Integration

```rust
// Guest exports
#[polkavm_derive::polkavm_export]
pub extern "C" fn execute(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
) {
    // Execute NeoVM script
}
```

---

## Error Handling

### Error Types

| Type | Source | Propagation |
|------|--------|-------------|
| VM Fault | Guest | State = FAULT, message in result |
| Out of Gas | Host | State = FAULT, "Insufficient GAS" |
| Host Error | C# | Propagated through callback |
| Internal | Runtime | Panic (bug) |

### Error Recovery

1. **VM Fault**: Clean shutdown, fault message preserved
2. **Out of Gas**: Immediate halt, gas consumed reported
3. **Host Error**: Error message returned, execution continues if recoverable
4. **Panic**: Host catches, returns generic error

---

## Future Extensions

### Native RISC-V Contracts

Contracts compiled directly to RISC-V (not NeoVM bytecode):
```
Contract Source
      │
      ▼ Compile
RISC-V Binary (guest.polkavm format)
      │
      ▼ Deploy
Execute directly (no NeoVM interpreter)
```

**Benefits:**
- 10x+ performance for compute-heavy contracts
- Access to RISC-V ecosystem
- Smaller binary size

### Multi-VM Support

Support for multiple VM types:
```
RiscvApplicationEngine
├── NeoVM path (current)
├── Native RISC-V path (future)
└── WASM path (possible)
```

---

## References

- [NEP Specification](./NEP-RISC-V-VM.md)
- [Testing Guide](./TESTING.md)
- [Deployment Guide](./DEPLOYMENT.md)
- [Optimization Plan](./OPTIMIZATION_PLAN.md)
