# API Reference

**Version:** 1.0  
**Last Updated:** 2026-03-24

---

## Table of Contents

1. [Rust API](#rust-api)
2. [C FFI API](#c-ffi-api)
3. [C# API](#c-api)
4. [Host Functions](#host-functions)
5. [Data Types](#data-types)

---

## Rust API

### `neo_riscv_host` Crate

#### Core Functions

```rust
/// Execute a script with default context
pub fn execute_script(script: &[u8]) -> Result<ExecutionResult, String>

/// Execute with trigger type
pub fn execute_script_with_trigger(
    script: &[u8], 
    trigger: u8
) -> Result<ExecutionResult, String>

/// Execute with full context
pub fn execute_script_with_context(
    script: &[u8],
    context: RuntimeContext,
) -> Result<ExecutionResult, String>

/// Execute with custom host callback
pub fn execute_script_with_host<F>(
    script: &[u8],
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(u32, usize, RuntimeContext, &[StackValue]) 
        -> Result<HostCallbackResult, String>;

/// Execute with initial stack
pub fn execute_script_with_host_and_stack<F>(
    script: &[u8],
    initial_stack: Vec<StackValue>,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
```

#### Types

```rust
/// Execution context
#[derive(Clone, Copy)]
pub struct RuntimeContext {
    /// Trigger type (0x00=Verification, 0x10=Application, etc.)
    pub trigger: u8,
    
    /// Network magic number
    pub network: u32,
    
    /// Address version byte
    pub address_version: u8,
    
    /// Block timestamp (optional)
    pub timestamp: Option<u64>,
    
    /// Remaining gas (datoshi)
    pub gas_left: i64,
    
    /// Fee factor in pico-units
    pub exec_fee_factor_pico: i64,
}

/// Execution result
pub struct ExecutionResult {
    /// VM state after execution
    pub state: VmState,
    
    /// Result stack
    pub stack: Vec<StackValue>,
    
    /// Gas consumed (pico-datosi)
    pub fee_consumed_pico: i64,
    
    /// Fault message (if state is FAULT)
    pub fault_message: Option<String>,
}

/// VM states
pub enum VmState {
    /// Execution completed successfully
    Halt = 0,
    
    /// Execution faulted
    Fault = 1,
    
    /// Execution paused (BREAK instruction)
    Break = 2,
}

/// Host callback result
pub struct HostCallbackResult {
    /// New stack after syscall
    pub stack: Vec<StackValue>,
}
```

#### Profiling Functions

```rust
/// Reset profiling counters
pub fn reset_profiling()

/// Get current memory usage (bytes)
pub fn get_current_memory() -> usize

/// Get peak memory usage (bytes)
pub fn get_peak_memory() -> usize
```

---

## C FFI API

### Exported Functions

#### `neo_riscv_execute_script`

```c
/**
 * Execute a script with default context
 * 
 * @param script - Script bytecode
 * @param script_len - Script length in bytes
 * @param result - Output execution result
 * @return true on success, false on error
 */
bool neo_riscv_execute_script(
    const uint8_t* script,
    size_t script_len,
    NativeExecutionResult* result
);
```

#### `neo_riscv_execute_script_with_host`

```c
/**
 * Execute a script with custom host callbacks
 * 
 * @param script - Script bytecode
 * @param script_len - Script length
 * @param initial_ip - Initial instruction pointer
 * @param trigger - Execution trigger type
 * @param network_magic - Network magic number
 * @param address_version - Address version byte
 * @param persisting_timestamp - Block timestamp
 * @param gas_left - Initial gas (datoshi)
 * @param exec_fee_factor_pico - Fee factor
 * @param initial_stack - Initial stack items
 * @param initial_stack_len - Number of stack items
 * @param user_data - Opaque pointer passed to callbacks
 * @param host_callback - Host callback function
 * @param host_free - Cleanup callback function
 * @param result - Output execution result
 * @return true on success, false on error
 */
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
    NativeHostCallback* host_callback,
    NativeHostFreeCallback* host_free,
    NativeExecutionResult* result
);
```

#### `neo_riscv_free_execution_result`

```c
/**
 * Free memory allocated for execution result
 * 
 * @param result - Result to free
 */
void neo_riscv_free_execution_result(
    NativeExecutionResult* result
);
```

### Types

```c
/// Execution result from native code
typedef struct {
    /// Gas consumed in pico-datosi
    int64_t fee_consumed_pico;
    
    /// VM state (0=HALT, 1=FAULT, 2=BREAK)
    uint32_t state;
    
    /// Pointer to stack items
    void* stack_ptr;
    
    /// Number of stack items
    size_t stack_len;
    
    /// Pointer to error message (if faulted)
    void* error_ptr;
    
    /// Length of error message
    size_t error_len;
} NativeExecutionResult;

/// Stack item kind
typedef enum {
    STACK_ITEM_INTEGER = 0,
    STACK_ITEM_BYTE_STRING = 1,
    STACK_ITEM_NULL = 2,
    STACK_ITEM_BOOLEAN = 3,
    STACK_ITEM_ARRAY = 4,
    STACK_ITEM_BIG_INTEGER = 5,
    STACK_ITEM_ITERATOR = 6,
    STACK_ITEM_STRUCT = 7,
    STACK_ITEM_MAP = 8,
    STACK_ITEM_INTEROP = 9,
    STACK_ITEM_POINTER = 10,
} NativeStackItemKind;

/// Stack item
typedef struct {
    /// Item kind
    uint32_t kind;
    
    /// For Integer: value; for Boolean: 0 or 1
    int64_t integer_value;
    
    /// For ByteString/Array/Map: data pointer
    void* bytes_ptr;
    
    /// For ByteString: length; for Array/Map: item count
    size_t bytes_len;
} NativeStackItem;

/// Host callback function
typedef bool (*NativeHostCallback)(
    void* user_data,
    uint32_t api,
    size_t instruction_pointer,
    uint8_t trigger,
    uint32_t network_magic,
    uint8_t address_version,
    uint64_t persisting_timestamp,
    int64_t gas_left,
    void* input_stack,
    size_t input_stack_len,
    NativeHostResult* result
);

/// Host result
typedef struct {
    void* stack_ptr;
    size_t stack_len;
    void* error_ptr;
    size_t error_len;
} NativeHostResult;

/// Cleanup callback
typedef void (*NativeHostFreeCallback)(
    void* user_data,
    NativeHostResult* result
);
```

---

## C# API

### `Neo.SmartContract.RiscV` Namespace

#### `RiscvAdapterPlugin`

```csharp
/// <summary>
/// Neo plugin that registers the RISC-V application engine provider.
/// </summary>
public sealed class RiscvAdapterPlugin : Plugin
{
    public override string Name => "Neo.Riscv.Adapter";
    public override string Description => "RISC-V VM adapter for Neo";
    
    /// <summary>
    /// Constructor resolves the native host library and sets
    /// ApplicationEngine.Provider for the current process.
    /// </summary>
    public RiscvAdapterPlugin()
    
    /// <summary>
    /// Cleanup on plugin unload.
    /// </summary>
    public override void Dispose()
}
```

#### `RiscvApplicationEngine`

```csharp
/// <summary>
/// ApplicationEngine implementation using RISC-V backend.
/// </summary>
public sealed class RiscvApplicationEngine : ApplicationEngine
{
    /// <summary>
    /// Internal constructor. Instances are created through
    /// RiscvApplicationEngineProvider.
    /// </summary>
    internal RiscvApplicationEngine(
        TriggerType trigger,
        IVerifiable? container,
        DataCache snapshot,
        Block? persistingBlock,
        ProtocolSettings settings,
        long gas,
        IRiscvVmBridge bridge,
        IDiagnostic? diagnostic = null,
        JumpTable? jumpTable = null
    );
    
    /// <summary>
    /// Execute the script.
    /// </summary>
    public override VMState Execute();
}
```

#### `RiscvApplicationEngineProvider`

```csharp
/// <summary>
/// Provider that creates RISC-V application engines. Core now expects
/// ApplicationEngine.Provider to be supplied explicitly by this provider
/// or by the adapter plugin.
/// </summary>
public sealed class RiscvApplicationEngineProvider : IApplicationEngineProvider, IDisposable
{
    /// <summary>
    /// Creates a new engine instance.
    /// </summary>
    public ApplicationEngine Create(
        TriggerType trigger,
        IVerifiable? container,
        DataCache snapshot,
        Block? persistingBlock,
        ProtocolSettings settings,
        long gas,
        IDiagnostic? diagnostic,
        JumpTable jumpTable
    );
    
    /// <summary>
    /// Disposes the provider and bridge.
    /// </summary>
    public void Dispose();
}
```

#### `IRiscvVmBridge`

```csharp
/// <summary>
/// Bridge interface for RISC-V VM communication.
/// </summary>
public interface IRiscvVmBridge
{
    /// <summary>
    /// Execute a script or loaded contract stack through the bridge.
    /// </summary>
    RiscvExecutionResult Execute(RiscvExecutionRequest request);

    /// <summary>
    /// Execute a deployed contract via bridge-owned dispatch.
    /// </summary>
    RiscvExecutionResult ExecuteContract(
        ApplicationEngine engine,
        ContractState contract,
        string method,
        CallFlags flags,
        IReadOnlyList<StackItem> args
    );
}
```

---

## Host Functions

### Guest → Host Interface

The guest VM can call two host functions:

#### `host_on_instruction`

```rust
/// Called before executing each opcode
/// 
/// # Arguments
/// * `opcode` - The opcode about to execute
/// 
/// # Returns
/// * 1 - Continue execution
/// * 0 - Abort (gas exhausted)
fn host_on_instruction(opcode: u32) -> u32;
```

**Purpose:** Gas accounting and instruction tracing

#### `host_call`

```rust
/// Called for SYSCALL opcode
/// 
/// # Arguments
/// * `api` - Syscall API hash
/// * `ip` - Current instruction pointer
/// * `stack_ptr` - Pointer to serialized stack
/// * `stack_len` - Length of serialized stack
/// * `result_ptr` - Pointer to result buffer
/// * `result_cap` - Capacity of result buffer
/// 
/// # Returns
/// * >0 - Success, bytes written to result
/// * 0 - Error
fn host_call(
    api: u32,
    ip: u32,
    stack_ptr: u32,
    stack_len: u32,
    result_ptr: u32,
    result_cap: u32,
) -> u32;
```

**Purpose:** Execute native contracts and system calls

---

## Data Types

### StackValue

The ABI supports these stack value types:

```rust
pub enum StackValue {
    /// 64-bit signed integer
    Integer(i64),
    
    /// Variable-length byte string
    ByteString(Vec<u8>),
    
    /// Big integer (up to 256 bits)
    BigInteger(Vec<u8>),
    
    /// Boolean value
    Boolean(bool),
    
    /// Null value
    Null,
    
    /// Array of items
    Array(Vec<StackValue>),
    
    /// Struct (value semantics)
    Struct(Vec<StackValue>),
    
    /// Map of key-value pairs
    Map(Vec<(StackValue, StackValue)>),
    
    /// Interop handle
    Interop(u64),
    
    /// Iterator handle
    Iterator(u64),
    
    /// Instruction pointer
    Pointer(usize),
}
```

### Binary Encoding

Stack values are encoded as:

```
[tag: u8] [data: variable]

Tags:
0x00 = Integer (8 bytes, little-endian i64)
0x01 = ByteString (4 bytes LE length + data)
0x02 = Boolean (1 byte: 0=false, 1=true)
0x03 = Null (no data)
0x04 = Array (4 bytes LE count + items)
0x05 = BigInteger (4 bytes LE length + data)
0x06 = Struct (4 bytes LE count + items)
0x07 = Map (4 bytes LE count + key-value pairs)
0x08 = Interop (8 bytes handle)
0x09 = Iterator (8 bytes handle)
0x0A = Pointer (8 bytes usize)
```

### Example Encodings

| Value | Encoding (hex) |
|-------|----------------|
| Integer(42) | `00 2a 00 00 00 00 00 00 00` |
| Boolean(true) | `02 01` |
| Null | `03` |
| ByteString("hi") | `01 02 00 00 00 68 69` |
| Array([1, 2]) | `04 02 00 00 00 00 01 00 00 00 00 00 00 00 00 02 00 00 00 00 00 00 00` |

---

## Error Codes

### VM Faults

| Fault | Message | Cause |
|-------|---------|-------|
| StackOverflow | "stack overflow" | Stack exceeds 2048 items |
| StackUnderflow | "stack underflow" | Pop from empty stack |
| OutOfGas | "Insufficient GAS" | Gas exhausted |
| InvalidOpcode | "unknown opcode" | Unrecognized opcode |
| InvalidJump | "invalid jump target" | JMP to invalid address |
| DivideByZero | "division by zero" | DIV/MOD with 0 |

### FFI Errors

| Error | Cause |
|-------|-------|
| Library not found | `libneo_riscv_host.so` not found |
| Symbol not found | Version mismatch |
| Memory allocation | Out of memory |
| Invalid parameter | Null pointer, etc. |

---

## Usage Examples

### Rust Example

```rust
use neo_riscv_host::{
    execute_script_with_context, 
    RuntimeContext, 
    HostCallbackResult
};

fn main() {
    let script = vec![0x11, 0x12, 0x9e, 0x40]; // PUSH1, PUSH2, ADD, RET
    
    let result = execute_script_with_context(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: Some(1710000000),
            gas_left: 10000000,
            exec_fee_factor_pico: 300000,
        },
    ).expect("execution failed");
    
    println!("State: {:?}", result.state);
    println!("Stack: {:?}", result.stack);
    println!("Gas consumed: {} datoshi", result.fee_consumed_pico / 10000);
}
```

### C Example

```c
#include <neo_riscv.h>
#include <stdio.h>

int main() {
    uint8_t script[] = {0x11, 0x12, 0x9e, 0x40};
    NativeExecutionResult result;
    
    bool success = neo_riscv_execute_script(
        script, 
        sizeof(script), 
        &result
    );
    
    if (success && result.state == 0) {
        printf("Execution succeeded\n");
        printf("Gas consumed: %ld\n", result.fee_consumed_pico);
    }
    
    neo_riscv_free_execution_result(&result);
    return 0;
}
```

### C# Example

```csharp
using Neo.SmartContract;

// Plugin auto-registers, use standard API
var engine = ApplicationEngine.Create(
    TriggerType.Application,
    container,
    snapshot,
    block,
    settings,
    gas: 10000000
);

var state = engine.Execute();
Console.WriteLine($"Execution state: {state}");
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-03-24 | Initial release |

---

## See Also

- [Architecture](./ARCHITECTURE.md)
- [NEP Specification](./NEP-RISC-V-VM.md)
- [Testing Guide](./TESTING.md)
- [Deployment Guide](./DEPLOYMENT.md)
