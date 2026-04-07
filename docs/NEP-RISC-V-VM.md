# NEP: RISC-V Virtual Machine for Neo N3

**Status:** Production Ready  
**Version:** 1.0  
**Author:** Neo RISC-V Team  
**Created:** 2026-03-24

---

## Abstract

This NEP specifies the RISC-V Virtual Machine stack used in the current Neo N3 integration workspace, with an internal NeoVM compatibility layer for backward-compatible execution.

## Motivation

1. **Security**: Sandboxed execution via PolkaVM
2. **Extensibility**: RISC-V ecosystem for future optimizations
3. **Compatibility**: Preserve existing contract semantics while moving the bridge layer into an external adapter
4. **Performance**: Optimizable execution with JIT potential

## Specification

### 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: Neo Core (C#)                                          │
│ - ApplicationEngine + provider hook                             │
│ - Native contracts                                              │
│ - Syscall interface                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Plugin Interface
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: Adapter (C#)                                           │
│ - RiscvAdapterPlugin                                            │
│ - IApplicationEngineProvider                                    │
│ - FFI Bridge                                                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ C FFI
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: Host Runtime (Rust)                                    │
│ - PolkaVM Engine                                                │
│ - Memory Management (256MB arena)                              │
│ - Host Callbacks                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Internal Syscalls
┌─────────────────────────────────────────────────────────────────┐
│ Layer 4: Guest VM (RISC-V)                                      │
│ - NeoVM Interpreter                                             │
│ - Stack Management                                              │
│ - Opcode Execution                                              │
└─────────────────────────────────────────────────────────────────┘
```

### 2. Execution Model

#### 2.1 Contract Execution Flow

1. **Invocation**: Neo core receives contract execution request
2. **Routing**: Adapter plugin or tests provide `ApplicationEngine.Provider`
3. **Preparation**: Host serializes script and initial stack
4. **Execution**: Guest interprets NeoVM bytecode
5. **Syscalls**: Guest calls host for native operations
6. **Completion**: Result returned through FFI to C#

#### 2.2 State Management

| Component | Responsibility |
|-----------|----------------|
| C# Core | Chain state, storage, native contracts |
| Host Runtime | VM state, memory, gas tracking |
| Guest VM | Execution context, stack, instruction pointer |

### 3. Memory Model

#### 3.1 Guest Memory Layout

```
┌─────────────────────────────────────────────────────┐ 0x0000_0000
│ Reserved (entry points, trampolines)               │
├─────────────────────────────────────────────────────┤
│ Code (guest.polkavm, read-only)                    │
├─────────────────────────────────────────────────────┤
│ Heap (bump allocator, 256MB max)                   │
│ - Stack values                                      │
│ - Compound types (Array, Map, Struct)              │
│ - Temporary allocations                            │
├─────────────────────────────────────────────────────┤
│ Aux Data (script + initial stack)                  │
├─────────────────────────────────────────────────────┤
│ Stack (execution stack, grows down)                │
└─────────────────────────────────────────────────────┘ 0xFFFF_FFFF
```

#### 3.2 Memory Limits

| Resource | Limit | Behavior on Exceed |
|----------|-------|-------------------|
| Heap | 256 MB | Allocation fails, guest faults |
| Stack Items | 2048 | Stack overflow error |
| Item Size | 1 MB | Size limit error |
| Call Depth | 1024 | Call depth exceeded |

### 4. Opcode Implementation

#### 4.1 Supported Opcodes

All NeoVM opcodes (0x00-0xFF) are supported:

| Category | Opcodes | Status |
|----------|---------|--------|
| Push | 0x00-0x21 | ✅ Full |
| Control Flow | 0x22-0x3F | ✅ Full |
| Stack | 0x43-0x54 | ✅ Full |
| Slot | 0x56-0x87 | ✅ Full |
| Splice | 0x88-0x8E | ✅ Full |
| Bitwise | 0x90-0x98 | ✅ Full |
| Arithmetic | 0x99-0xBB | ✅ Full |
| Compound | 0xBE-0xD3 | ✅ Full |

#### 4.2 Opcode Semantics

Key implementation details:

- **CALL / CALL_L / CALLA**: Save the current locals and initialization state onto the call stack before jumping. A fresh locals array is allocated for the callee.
- **RET**: Restore the caller's saved locals and initialization state from the call stack.
- **ENDTRY / ENDTRY_L**: When a finally block exists, the continuation IP is saved in the try frame (`end_ip`) before entering the finally block. ENDFINALLY then jumps to the saved continuation IP.
- **JMPEQ / JMPNE**: Use `vm_equal` for value comparison, which handles cross-type equality (e.g., Integer vs BigInteger, ByteString vs Boolean) matching NeoVM semantics.
- **EQUAL / NOTEQUAL**: Also use `vm_equal` for consistent comparison behavior.

#### 4.3 Opcode Pricing

Gas costs match NeoVM exactly:

```rust
fn opcode_price(opcode: u8) -> i64 {
    match opcode {
        0x38 | 0x40 | 0x41 | 0xe0 => 0,           // Free
        0x00..=0x03 | 0x08 | 0x09 | 0x0b | 
        0x0f | 0x10..=0x21 | 0x39 | 0xe1 => 1,   // Cheapest
        0x22..=0x33 => 2,                          // Jumps
        // ... etc
        _ => 65536,                                // Default (expensive)
    }
}
```

### 5. Syscall Interface

#### 5.1 Host Functions

Two host functions are exposed to the guest:

```rust
// Called for each executed opcode (gas accounting)
fn host_on_instruction(opcode: u32) -> u32;

// Called for SYSCALL opcode
fn host_call(
    api: u32,           // Syscall API hash
    ip: u32,            // Instruction pointer
    stack_ptr: u32,     // Stack data pointer
    stack_len: u32,     // Stack data length
    result_ptr: u32,    // Result buffer pointer
    result_cap: u32,    // Result buffer capacity
) -> u32;              // Returns: bytes written or 0 on error
```

#### 5.2 Standard Syscalls

All standard NeoVM syscalls are supported:

| Syscall | API Hash | Status |
|---------|----------|--------|
| System.Runtime.Platform | 0xC15B8F... | ✅ |
| System.Runtime.GetTrigger | 0xE7F6C6... | ✅ |
| System.Runtime.GetNetwork | 0xA8A222... | ✅ |
| System.Storage.Get | 0xAF9E1C... | ✅ |
| System.Storage.Put | 0x395C4A... | ✅ |
| ... | ... | ✅ All |

### 6. ABI Specification

#### 6.1 Stack Value Encoding

Stack values are serialized using a compact binary format:

```
StackValue:
  | Tag (1 byte) | Data (variable) |

Tags (fast_codec):
  0x01 = Integer (8 bytes, little-endian i64)
  0x02 = BigInteger (4 bytes length + data)
  0x03 = ByteString (4 bytes length + data)
  0x04 = Boolean (1 byte: 0 or 1)
  0x05 = Array (4 bytes count + items, recursive)
  0x06 = Struct (4 bytes count + items, recursive)
  0x07 = Map (4 bytes count + key-value pairs, recursive)
  0x08 = Interop (8 bytes handle)
  0x09 = Iterator (8 bytes handle)
  0x0A = Null (no data)
  0x0B = Pointer (8 bytes i64)
```

#### 6.2 Execution Result

```rust
struct ExecutionResult {
    state: VmState,          // HALT, FAULT, BREAK
    stack: Vec<StackValue>,  // Result stack
    gas_consumed: i64,       // Gas consumed in datoshi
    exception: Option<String>, // Fault message
}
```

### 7. FFI Specification

#### 7.1 C Interface

```c
// Execute script with host callbacks
typedef struct {
    uint64_t fee_consumed_pico;
    uint32_t state;
    void* stack_ptr;
    size_t stack_len;
    void* error_ptr;
    size_t error_len;
} NativeExecutionResult;

bool neo_riscv_execute_script_with_host(
    const uint8_t* script,
    size_t script_len,
    size_t initial_ip,
    uint8_t trigger,
    uint32_t network_magic,
    uint8_t address_version,
    uint64_t persisting_timestamp,
    int64_t gas_left,
    int64_t exec_fee_factor_pico,
    void* initial_stack,
    size_t initial_stack_len,
    void* user_data,
    void* host_callback,
    void* host_free,
    NativeExecutionResult* result
);

void neo_riscv_free_execution_result(NativeExecutionResult* result);
```

### 8. Error Handling

#### 8.1 Error Categories

| Category | Description | Handling |
|----------|-------------|----------|
| VM Fault | Execution error (stack overflow, etc.) | Returns FAULT state |
| Host Error | Syscall failure | Returns error message |
| Out of Gas | Gas exhausted | Returns FAULT with "Insufficient GAS" |
| Internal Error | VM internal failure | Panics (should not happen) |

#### 8.2 Error Propagation

Errors propagate cleanly across the FFI boundary:

```
Guest VM error → Host encodes → FFI returns → C# exception
```

### 9. Security Considerations

#### 9.1 Sandboxing

- Guest runs in PolkaVM sandbox
- No direct host memory access
- All memory access bounds-checked
- Syscalls validated by API hash

#### 9.2 Resource Limits

| Resource | Limit | Enforcement |
|----------|-------|-------------|
| Memory | 256 MB | Hard limit |
| Execution time | Gas-based | Indirect limit |
| Call depth | 1024 | Hard limit |
| Stack size | 2048 items | Hard limit |
| Guest result size | 16 MB (`MAX_RESULT_SIZE`) | Hard limit per host read |
| Instance pool | 16 per aux-data size (`MAX_POOL_SIZE_PER_AUX`) | Bounded pool cap |

#### 9.3 Codec Safety

The custom fast codec (`fast_codec.rs`) enforces defensive limits during deserialization:

| Guard | Value | Purpose |
|-------|-------|---------|
| `MAX_DECODE_DEPTH` | 64 | Prevents stack overflow from deeply nested structures |
| `MAX_COLLECTION_LEN` | 4096 | Prevents OOM from oversized arrays, maps, or structs |

These limits apply to both the top-level stack length and every nested collection. Payloads that exceed either limit are rejected with an error before any allocation occurs.

### 10. Compatibility

#### 10.1 NeoVM Compatibility

The RISC-V VM is 100% compatible with NeoVM:

- All opcodes produce identical results
- Gas costs match exactly
- Error messages match
- Stack behavior identical

#### 10.2 Migration Path

**Phase 1**: Workspace adapter integration (current)
- Externalized adapter owns the bridge/provider code
- Core stays generic but is no longer literally unchanged
- Node integration is validated with the packaged plugin bundle

**Phase 2**: Reduced workspace coupling (future)
- Eliminate sibling-project assumptions in core tests and packaging
- Preserve plugin-based deployment

**Phase 3**: Native RISC-V (optional)
- Contracts compile to RISC-V
- Maximum performance

### 11. Implementation Requirements

#### 11.1 Host Requirements

- PolkaVM 0.32.x
- Rust 1.70+
- Linux x86_64 (primary)

#### 11.2 Guest Requirements

- RISC-V 32-bit target
- No standard library (no_std)
- PolkaVM export metadata

#### 11.3 C# Requirements

- .NET 10.0+
- Neo 3.9.0+
- P/Invoke support

### 12. Testing

#### 12.1 Test Requirements

| Test Suite | Minimum | Target |
|------------|---------|--------|
| Unit Tests | 80% coverage | 95% coverage |
| Integration | All syscalls | All contract types |
| Compatibility | JSON corpus | Neo core tests |
| Performance | <50µs/op | <20µs/op |

#### 12.2 Validation Criteria

- All Neo core unit tests pass
- Gas costs match reference within 1%
- Memory usage bounded
- No memory leaks

### 13. References

- NeoVM Specification: https://github.com/neo-project/neo-vm
- PolkaVM Documentation: https://github.com/paritytech/polkavm
- RISC-V Specification: https://riscv.org/specifications/

### 14. Copyright

This NEP is released under the MIT License.
