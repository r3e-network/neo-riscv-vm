# Neo N3 RISC-V VM Architecture Design

## Vision

PolkaVM (RISC-V) as the sole execution engine for Neo N3. NeoVM becomes a
native contract that interprets NeoVM bytecode inside the RISC-V sandbox.
Existing NeoVM contracts, tools, and SDKs work unchanged. New contracts can
target RISC-V directly.

## Implementation Status: COMPLETE

### Rust Side (neo-riscv-vm) -- DONE

- `runtime_cache.rs` -- `compile_native_module()` compiles arbitrary PolkaVM binaries
- `lib.rs` -- `execute_native_contract()` core direct execution logic
- `ffi.rs` -- `neo_riscv_execute_native_contract()` C FFI entry point
- **141/141 tests passing, zero regressions**

### C# Side (Neo N3 fork) -- DONE

- `ContractType.cs` -- `ContractType` enum (NeoVM=0, RiscV=1)
- `ContractState.cs` -- `Type` field added, backward compatible
- `ContractManagement.cs` -- Auto-detects PVM\0 magic on deploy
- `ContractVmTypeResolver.cs` -- Resolves contract type from manifest/magic
- `IRiscvVmBridge.cs` -- Added `ExecuteContract()` method
- `NativeRiscvVmBridge.cs` -- Implemented `ExecuteContract()`
- `RiscvApplicationEngine.cs` -- `Execute()` collects ContractType per context
- `RiscvExecutionDispatcher` -- Routes NeoVM vs native RISC-V automatically

## Architecture

```
  RISC-V VM (Sole Execution Engine)
       |
  +----+-----------------------------+
  |                                  |
  NeoVM Contract                RISC-V Contract
  (ContractType.NeoVM=0)        (ContractType.RiscV=1)
  |                                  |
  PolkaVM loads guest.polkavm   PolkaVM loads contract binary
  (NeoVM interpreter)           (direct execution)
  |                                  |
  Interprets NeoVM bytecode     Executes RISC-V code
  |                                  |
  +----+-----------------------------+
       |
  Shared host_call bridge
  SYSCALL -> C# interop handlers
  CALLT -> native contract dispatch
```

## Execution Paths

### Path 1: NeoVM Contract (Existing, Unchanged)

```
ApplicationEngine.Run()
  -> RiscvApplicationEngine.Execute()
    -> Collects ContractType per InvocationStack context
    -> bridge.Execute(RiscvExecutionRequest)
      -> RiscvExecutionDispatcher detects ContractType.NeoVM
        -> PolkaVM loads guest.polkavm (cached NeoVM interpreter)
        -> Interpreter runs contract.Nef.Script (NeoVM bytecode)
        -> SYSCALL/CALLT -> host_call -> C# callback
      -> Result returned
```

### Path 2: Native RISC-V Contract (NEW)

```
ApplicationEngine.Run()
  -> RiscvApplicationEngine.Execute()
    -> Collects ContractType per InvocationStack context
    -> bridge.Execute(RiscvExecutionRequest)
      -> RiscvExecutionDispatcher detects ContractType.RiscV
        -> PolkaVM compiles & loads contract.Nef.Script (RISC-V binary)
        -> Direct execution (no interpreter layer)
        -> SYSCALL -> host_call -> C# callback
      -> Result returned
```

### Path 2b: Direct ExecuteContract (NEW convenience API)

```
bridge.ExecuteContract(engine, contract, method, flags, args)
  -> Constructs RiscvExecutionRequest from engine context
  -> Delegates to Execute() with proper ContractType routing
```

## ContractType

```csharp
public enum ContractType : byte
{
    NeoVM = 0,  // Default. NEF contains NeoVM bytecode.
    RiscV = 1,  // NEF contains PolkaVM binary (PVM\0 magic).
}
```

Stored in `ContractState`. Backward compatible: value 0 = NeoVM (existing
contracts have no explicit type field, default to 0).

## Rust FFI: Native Contract Execution

```rust
// New FFI entry point for direct RISC-V binary execution
pub unsafe extern "C" fn neo_riscv_execute_native_contract(
    binary_ptr: *const u8,      // RISC-V contract binary
    binary_len: usize,
    method_ptr: *const u8,      // method name (UTF-8)
    method_len: usize,
    initial_stack_ptr: *const NativeStackItem,
    initial_stack_len: usize,
    trigger: u8,
    network: u32,
    address_version: u8,
    timestamp: u64,
    gas_left: i64,
    exec_fee_factor_pico: i64,
    user_data: *mut c_void,
    callback: NativeHostCallback,
    free_callback: NativeHostFreeCallback,
    output: *mut NativeExecutionResult,
) -> bool
```

### Native Contract ABI Convention

- Contract must export: `execute(u32, u32)`, `get_result_ptr()`, `get_result_len()`
- Contract may import: `host_call` for syscalls, `host_on_instruction` for gas metering
- Method name prepended as ByteString to initial stack for dispatch
- Result serialized as postcard-encoded `ExecutionResult`

## Backward Compatibility

| Component                | Change Required | Impact                     |
| ------------------------ | --------------- | -------------------------- |
| Existing NeoVM contracts | None            | Same interpreter-on-RISC-V |
| ContractManagement       | Add Type field  | Default = NeoVM, additive  |
| System.Contract.Call     | Route by type   | Transparent to callers     |
| neo-boa / neon compiler  | None            | Produces NeoVM NEF         |
| neo-express              | None            | Deploys NEF as before      |
| SDKs (JS/Go/Python)      | None            | Same API                   |
| Wallets                  | None            | Same contract model        |
| RPC nodes                | None            | Same protocol              |
| Native contracts         | None            | Always NeoVM type          |
| P2P / consensus          | None            | Protocol unchanged         |

## Key Design Decisions

1. **ContractType in ContractState, not NEF header**: NEF format is
   standardized. Putting type in ContractState is a node-level concern.

2. **Same deploy/invoke**: Both types use `ContractManagement.Deploy()` and
   `System.Contract.Call`. The dispatch layer routes transparently.

3. **Shared host callback**: Both paths use the same SYSCALL/CALLT bridge.
   All interop works identically.

4. **No protocol change**: This is a node implementation detail. The network
   protocol, block format, and consensus are unchanged.

5. **PVM\0 magic detection**: Reliable identifier for PolkaVM binaries.
   NeoVM bytecode never starts with these bytes.

6. **Automatic routing via RiscvExecutionDispatcher**: No explicit branching
   in `ApplicationEngine.Contract.cs`. The dispatcher in the bridge handles
   ContractType routing transparently.

## Key Files

### Rust (neo-riscv-vm)

- `crates/neo-riscv-host/src/lib.rs` -- `execute_native_contract()`
- `crates/neo-riscv-host/src/ffi.rs` -- `neo_riscv_execute_native_contract()`
- `crates/neo-riscv-host/src/runtime_cache.rs` -- `compile_native_module()`
- `crates/neo-riscv-host/src/bridge.rs` -- shared `host_call` bridge
- `crates/neo-riscv-abi/src/lib.rs` -- `StackValue`, `ExecutionResult`
- `crates/neo-riscv-guest/src/lib.rs` -- NeoVM interpreter (guest)
- `crates/neo-riscv-guest-module/src/main.rs` -- guest binary entry

### C# (Neo N3 fork)

- `src/Neo/SmartContract/ContractType.cs` -- enum
- `src/Neo/SmartContract/ContractState.cs` -- Type field
- `src/Neo/SmartContract/ContractVmTypeResolver.cs` -- PVM\0 detection
- `src/Neo/SmartContract/RiscV/IRiscvVmBridge.cs` -- interface
- `src/Neo/SmartContract/RiscV/NativeRiscvVmBridge.cs` -- implementation
- `src/Neo/SmartContract/RiscV/RiscvApplicationEngine.cs` -- Execute() override
- `src/Neo/SmartContract/Native/ContractManagement.cs` -- Deploy with type
