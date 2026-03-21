# NEP-RISCV: RISC-V Virtual Machine for Neo N3

## Status

Draft

## Abstract

This proposal introduces RISC-V virtual machine support for Neo N3 through
PolkaVM, a sandboxed RISC-V runtime by Parity Technologies. The proposal
enables two contract execution backends on a unified infrastructure:

1. **NeoVM contracts**: Existing NeoVM bytecode executed by a NeoVM interpreter
   running as a guest program inside the PolkaVM sandbox. Fully backward
   compatible with all existing Neo N3 contracts, tools, and SDKs.

2. **RISC-V contracts**: Native RISC-V binaries executed directly by PolkaVM,
   providing near-native performance for compute-intensive workloads.

Both contract types share the same deployment model (`ContractManagement.Deploy`),
the same invocation model (`System.Contract.Call`), the same manifest format,
the same gas metering infrastructure, and the same SYSCALL interop surface.
The distinction is made through a `ContractType` field in `ContractState`,
automatically detected at deployment time from the NEF script prefix.

## Motivation

Neo N3's current NeoVM, while capable, has performance limitations for
compute-heavy operations (cryptographic primitives, data processing, complex
algorithms). Running these operations in a sandboxed RISC-V environment provides:

- **Performance**: Near-native execution speed for compiled RISC-V binaries
- **Language flexibility**: Smart contracts can be written in Rust, C, C++, or
  any language targeting RISC-V
- **Sandboxing**: PolkaVM provides memory isolation, deterministic execution,
  and syscall-based host interaction
- **Backward compatibility**: Existing NeoVM contracts continue to work through
  the interpreter-on-PolkaVM bridge, requiring zero changes

## Specification

### 1. ContractType Enum

```csharp
namespace Neo.SmartContract
{
    /// <summary>
    /// Defines the execution backend for a deployed contract.
    /// </summary>
    public enum ContractType : byte
    {
        /// <summary>
        /// NeoVM bytecode executed by the NeoVM interpreter guest inside PolkaVM.
        /// This is the default. All existing contracts have this type.
        /// </summary>
        NeoVM = 0,

        /// <summary>
        /// PolkaVM RISC-V binary executed directly by PolkaVM.
        /// The NEF script begins with the PVM\0 magic bytes (0x50 0x56 0x4D 0x00).
        /// </summary>
        RiscV = 1,
    }
}
```

### 2. ContractState Extension

The `ContractState` structure is extended with a `Type` field:

```csharp
public class ContractState : IInteroperableVerifiable
{
    public int Id;
    public ushort UpdateCounter;
    public ContractType Type;           // NEW: execution backend type
    public required UInt160 Hash;
    public required NefFile Nef;
    public required ContractManifest Manifest;
    public ReadOnlyMemory<byte> Script => Nef.Script;
}
```

#### Serialization

- **StackItem representation**: `Type` is serialized as the 6th element of the
  `Array` representation (after Id, UpdateCounter, Hash, Nef, Manifest).
  Value is `(int)(byte)Type`.

- **Backward compatibility**: During deserialization (`FromStackItem`), if the
  array has fewer than 6 elements, `Type` defaults to `ContractType.NeoVM`.
  This ensures existing serialized `ContractState` data remains valid.

- **JSON representation**: `Type` is serialized as a string field `"type"` with
  value `"NeoVM"` or `"RiscV"`.

#### Deterministic Hash

The contract hash is computed identically for both types:

```
hash = SHA160(0x00 || sender || nefCheckSum || name)
```

The `Type` field does NOT affect the contract hash. This ensures hash stability
across type changes during contract updates.

### 3. Binary Format Detection

A contract's execution type is determined by inspecting the NEF script prefix:

```csharp
private static bool IsRiscVBinary(ReadOnlyMemory<byte> script)
{
    var span = script.Span;
    return span.Length > 4
        && span[0] == 0x50  // 'P'
        && span[1] == 0x56  // 'V'
        && span[2] == 0x4D  // 'M'
        && span[3] == 0x00; // '\0'
}
```

The PVM\0 magic bytes identify PolkaVM binary format. NeoVM bytecode never
begins with these bytes, providing unambiguous detection.

### 4. Deployment

#### ContractManagement.Deploy(nefFile, manifest)

1. Deserialize NEF file
2. Parse and validate manifest
3. Detect binary type: `IsRiscVBinary(nef.Script)` → `ContractType.RiscV`
   or `ContractType.NeoVM`
4. Compute contract hash (unchanged)
5. Create `ContractState` with detected `Type`
6. Store in persistent storage

#### ContractManagement.Update(nefFile, manifest)

1. Load existing `ContractState`
2. If NEF is updated: re-detect type from new NEF script
3. If manifest only is updated: retain existing `Type`
4. Store updated `ContractState`

#### Manifest Requirements

Both contract types use the SAME manifest format:

```json
{
    "name": "MyContract",
    "groups": [],
    "features": {},
    "supportedstandards": [],
    "abi": {
        "methods": [
            {
                "name": "transfer",
                "parameters": [
                    { "name": "from", "type": "Hash160" },
                    { "name": "to", "type": "Hash160" },
                    { "name": "amount", "type": "Integer" },
                    { "name": "data", "type": "Any" }
                ],
                "returntype": "Boolean",
                "offset": 0,
                "safe": false
            }
        ],
        "events": []
    },
    "permissions": [{ "contract": "*", "methods": "*" }],
    "trusts": [],
    "extra": null
}
```

For RISC-V contracts, the `"offset"` field in ABI methods refers to the
entry point within the RISC-V binary (typically 0 for the main function,
or a symbol offset for named methods).

### 5. Invocation

#### System.Contract.Call(hash, method, flags, args)

```csharp
protected internal void CallContract(
    UInt160 contractHash, string method, CallFlags callFlags, Array args)
{
    // Validation (unchanged)
    if (method.StartsWith('_')) throw ...;
    if ((callFlags & ~CallFlags.All) != 0) throw ...;

    // Contract lookup (unchanged)
    ContractState? contract = ContractManagement.GetContract(
        SnapshotCache, contractHash);
    if (contract is null) throw ...;
    ContractMethodDescriptor? md = contract.Manifest.Abi
        .GetMethod(method, args.Count);
    if (md is null) throw ...;

    // NEW: Route by contract type
    if (contract.Type == ContractType.RiscV)
    {
        ExecuteRiscVContract(contract, method, callFlags, args);
        return;
    }

    // EXISTING: NeoVM path (unchanged)
    bool hasReturnValue = md.ReturnType != ContractParameterType.Void;
    ExecutionContext context = CallContractInternal(
        contract, md, callFlags, hasReturnValue, args);
    context.GetState<ExecutionContextState>().IsDynamicCall = true;
}
```

#### NeoVM Contract Execution Path

```
System.Contract.Call(neoVmHash, method, args)
  → ContractState.Type == NeoVM
  → CallContractInternal()
    → LoadContract()
      → RiscvApplicationEngine.Execute()
        → Bridge.Execute(request)
          → PolkaVM loads guest.polkavm (NeoVM interpreter)
          → Interpreter executes contract.Nef.Script
          → SYSCALL → host callback → C# interop handler
          → Result returned
        → Result pushed to engine stack
      → Gas consumed
```

#### RISC-V Contract Execution Path

```
System.Contract.Call(riscvHash, method, args)
  → ContractState.Type == RiscV
  → ExecuteRiscVContract()
    → Bridge.ExecuteContract(engine, contract, method, flags, args)
      → PolkaVM loads contract.Nef.Script (RISC-V binary)
      → Method name + args passed via initial stack
      → Guest executes RISC-V code
      → SYSCALL → host callback → C# interop handler (same 40 SYSCALLs)
      → Result returned
    → Result pushed to engine stack
    → Gas consumed
```

Both paths converge at the host callback layer. The SYSCALL interop surface
is identical for both contract types.

### 6. SYSCALL Interface

All 40 registered SYSCALL interop calls are available to both contract types
through the host callback mechanism:

#### System.Runtime (18 calls)

| SYSCALL Hash | Name                                  | Description                |
| ------------ | ------------------------------------- | -------------------------- |
| 0x377b165f   | System.Runtime.Platform               | Returns "NEO"              |
| 0x7930cc02   | System.Runtime.GetTrigger             | Returns trigger type       |
| 0x44a15b0e   | System.Runtime.GetNetwork             | Returns network magic      |
| 0x52804004   | System.Runtime.GetAddressVersion      | Returns address version    |
| 0x2a09af62   | System.Runtime.GasLeft                | Returns remaining gas      |
| 0x2a80d983   | System.Runtime.GetRandom              | Returns random value       |
| 0x84e0968d   | System.Runtime.GetScriptContainer     | Returns transaction/Block  |
| 0xdb739d8c   | System.Runtime.GetExecutingScriptHash | Current script hash        |
| 0x2da3b086   | System.Runtime.GetCallingScriptHash   | Caller script hash         |
| 0x56d28c34   | System.Runtime.GetEntryScriptHash     | Entry script hash          |
| 0x47820449   | System.Runtime.LoadScript             | Load and execute script    |
| 0xf46a5458   | System.Runtime.CheckWitness           | Verify witness             |
| 0x607d96a6   | System.Runtime.GetInvocationCounter   | Invocation counter         |
| 0x7b3bb713   | System.Runtime.Log                    | Emit log event             |
| 0x3b3e11fc   | System.Runtime.Notify                 | Emit notification          |
| 0x2da3b087   | System.Runtime.GetNotifications       | Get notifications          |
| 0xc03b03f4   | System.Runtime.BurnGas                | Burn gas                   |
| 0x6c5e0f87   | System.Runtime.CurrentSigners         | Current signers            |
| 0x8d3d1d07   | System.Runtime.GetTime                | Persisting block timestamp |

#### System.Storage (11 calls)

| SYSCALL Hash | Name                              | Description           |
| ------------ | --------------------------------- | --------------------- |
| 0x7b3bb713   | System.Storage.GetContext         | Get storage context   |
| 0xd2455448   | System.Storage.GetReadOnlyContext | Get read-only context |
| 0xf5a4c548   | System.Storage.AsReadOnly         | Convert to read-only  |
| 0x925de831   | System.Storage.Get                | Read storage value    |
| 0xe63f1884   | System.Storage.Put                | Write storage value   |
| 0x2f819b50   | System.Storage.Delete             | Delete storage key    |
| 0x82b3e878   | System.Storage.Find               | Iterate storage       |
| 0x8b3bb713   | System.Storage.Local.GetContext   | Local storage context |
| 0x925de832   | System.Storage.Local.Get          | Read local storage    |
| 0xe63f1885   | System.Storage.Local.Put          | Write local storage   |
| 0x2f819b51   | System.Storage.Local.Delete       | Delete local storage  |
| 0x82b3e879   | System.Storage.Local.Find         | Iterate local storage |

#### System.Contract (7 calls)

| SYSCALL Hash | Name                                  | Description             |
| ------------ | ------------------------------------- | ----------------------- |
| 0x626566e8   | System.Contract.Call                  | Call a contract         |
| 0x4a783e27   | System.Contract.CallNative            | Call native contract    |
| 0x2a09af63   | System.Contract.GetCallFlags          | Get call flags          |
| 0x5d313d2d   | System.Contract.CreateStandardAccount | Create standard account |
| 0x6a0b4b3d   | System.Contract.CreateMultisigAccount | Create multisig account |
| 0x2a09af64   | System.Contract.NativeOnPersist       | On persist hook         |
| 0x2a09af65   | System.Contract.NativePostPersist     | Post persist hook       |

#### System.Crypto (2 calls)

| SYSCALL Hash | Name                        | Description      |
| ------------ | --------------------------- | ---------------- |
| 0x5d313d2e   | System.Crypto.CheckSig      | Verify signature |
| 0x6a0b4b3e   | System.Crypto.CheckMultisig | Verify multisig  |

#### System.Iterator (2 calls)

| SYSCALL Hash | Name                  | Description       |
| ------------ | --------------------- | ----------------- |
| 0x7b3bb714   | System.Iterator.Next  | Advance iterator  |
| 0x7b3bb715   | System.Iterator.Value | Get current value |

### 7. Gas Metering

#### Opcode Pricing

Gas consumption is measured identically for both contract types:

- **NeoVM contracts**: Each NeoVM opcode is charged according to the
  `opcode_price()` table. The price is multiplied by `exec_fee_factor_pico`
  and deducted from `gas_left`.

- **RISC-V contracts**: PolkaVM instruction counting provides gas metering.
  Each RISC-V instruction is charged based on its complexity class.

Both paths use the same `charge_opcode` function and the same gas accounting
infrastructure. The `fee_consumed_pico` value is returned identically.

#### Execution Fee Factor

```
datoshi_consumed = ceiling(pico_datoshi_consumed / 10_000)
gas_consumed = datoshi_consumed * datoshi_per_gas
```

This formula is unchanged for both contract types.

### 8. Storage Model

Both contract types share the same storage namespace:

```
StorageKey = ContractManagement.Id + Prefix_Contract + contractHash
```

Storage operations (Get, Put, Delete, Find) work identically for both types.
The storage context is resolved by contract hash, independent of contract type.

### 9. Cross-Contract Calls

Both contract types can call each other:

```
NeoVM contract → System.Contract.Call(riscvHash, method, args)
  → Bridge executes RISC-V binary
  → RISC-V binary can issue SYSCALL System.Contract.Call(neoVmHash, ...)
    → Bridge re-enters NeoVM interpreter
    → NeoVM contract executes
    → Result returned to RISC-V binary
  → Result returned to original NeoVM contract

RISC-V contract → System.Contract.Call(neoVmHash, method, args)
  → Bridge enters NeoVM interpreter
  → NeoVM contract executes
  → Result returned to RISC-V binary
```

Gas is properly accounted across nested calls. Each bridge invocation tracks
consumed gas and reports it back to the caller.

### 10. Transaction Model

The transaction model is UNCHANGED. Transactions contain:

```
Transaction {
    Version: byte
    Nonce: uint
    Sender: UInt160
    SystemFee: long
    NetworkFee: long
    ValidUntilBlock: uint
    Signers: Signer[]
    Attributes: TransactionAttribute[]
    Script: byte[]          // Invocation script
    Witnesses: Witness[]
}
```

The `Script` field contains the invocation script (typically `SYSCALL
System.Contract.Call` with the target contract hash, method, and arguments).
This is identical for both contract types — the caller does not need to know
the target's execution type.

### 11. Witness Verification

Witness verification is UNCHANGED:

```
Witness {
    InvocationScript: byte[]    // Signature
    VerificationScript: byte[]  // Public key check script
}
```

Verification scripts are NeoVM bytecode (always). Witness verification
always runs through the NeoVM interpreter, regardless of the contract
being invoked.

### 12. NEF Format

The NEF (Neo Executable File) format is UNCHANGED:

```
NefFile {
    Magic: uint32           // 0x3346454E ("NEF3")
    Compiler: string[64]    // Compiler identifier
    Source: string[256]     // Source URL
    Tokens: MethodToken[]   // External method references
    Reserve: byte[2]        // Reserved
    Script: byte[]          // Contract bytecode (NeoVM) or binary (RISC-V)
    Checksum: uint32        // CRC32 checksum
}
```

For RISC-V contracts:

- `Compiler` should identify the RISC-V compiler (e.g., "rustc-riscv-1.x")
- `Script` contains the PolkaVM binary (starts with PVM\0 magic)
- `Tokens` may reference external contracts called via SYSCALL

### 13. Security Considerations

#### Sandbox Isolation

PolkaVM provides memory isolation between guest programs and the host.
Each contract execution runs in a separate PolkaVM instance with:

- Fixed memory arena (4MB default)
- No direct memory access to host
- All host interaction through SYSCALL callbacks
- Deterministic execution (no floating point, no system calls)

#### Gas Limit Enforcement

Gas is enforced at two levels:

1. **Opcode level**: Each instruction is charged before execution
2. **Host callback level**: Each SYSCALL is charged by the host

If gas is exhausted, execution terminates with an `Insufficient GAS` fault.
This is identical for both contract types.

#### Determinism

Both execution paths are deterministic:

- **NeoVM**: The interpreter is deterministic by design
- **RISC-V**: PolkaVM disables non-deterministic features (floating point,
  system calls). `System.Runtime.GetRandom` is provided through the SYSCALL
  interface with deterministic seeding.

### 14. Backward Compatibility

| Component          | Change        | Impact                              |
| ------------------ | ------------- | ----------------------------------- |
| Transaction format | None          | Identical wire format               |
| Witness format     | None          | Identical                           |
| NEF format         | None          | Identical (script content varies)   |
| Manifest format    | None          | Identical                           |
| SYSCALL surface    | None          | Identical (40/40)                   |
| Native contracts   | None          | Identical behavior                  |
| Gas model          | None          | Identical accounting                |
| Storage model      | None          | Identical namespace                 |
| P2P protocol       | None          | Identical                           |
| Consensus          | None          | Identical                           |
| RPC interface      | None          | Identical                           |
| ContractState      | +1 byte field | Backward compatible (default NeoVM) |

### 15. Reference Implementation

| Component               | Repository      | Branch                         |
| ----------------------- | --------------- | ------------------------------ |
| RISC-V VM (Rust)        | neo-riscv-vm    | main                           |
| Neo Core changes        | jim8y/neo       | feature/riscv-native-contracts |
| PolkaVM runtime         | polkavm v0.32.0 | (dependency)                   |
| NeoVM interpreter guest | neo-riscv-guest | (compiled to guest.polkavm)    |

#### Test Coverage

- Rust VM tests: 141/141 passing
- C# integration tests: 983/983 passing
- Native contract tests: 175/175 passing
- SYSCALL coverage: 40/40 registered interops

### 16. Migration Path

1. **Phase 1** (Current): Deploy updated node with RISC-V bridge.
   Existing NeoVM contracts work transparently through interpreter-on-PolkaVM.

2. **Phase 2**: ContractType system deployed. Existing contracts default to
   NeoVM type. New RISC-V contracts can be deployed.

3. **Phase 3**: Ecosystem tools (compilers, SDKs) add RISC-V target support.
   Developers can choose NeoVM or RISC-V for new contracts.

No existing contract requires migration. No existing tool requires update.
The transition is additive only.
