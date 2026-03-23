# Neo RISC-V Contract Examples

⚠️ **ARCHITECTURE NOTE**: Contracts call syscalls through the Rust devpack, which forwards into the Neo N3 C# engine via FFI. Syscall and native contract semantics remain defined by C#.

## Available Examples

1. **counter** - State management with increment/get/reset
2. **storage** - Storage operations (put/get/delete)
3. **hello-world** - Minimal contract template
4. **nep17-token** - NEP-17 fungible token standard

## Supported Neo N3 Syscalls

### System.Contract

- `contract_call` - Call another contract
- `contract_create` - Deploy new contract
- `contract_update` - Update contract code

### System.Storage

- `storage::get` - Read from storage
- `storage::put` - Write to storage
- `storage::delete` - Delete from storage

### System.Runtime

- `runtime_notify` - Emit event
- `runtime_log` - Write log
- `runtime_check_witness` - Verify witness

### System.Crypto

- `crypto_verify_signature` - Verify signature

## Build & Test

```bash
./tests/e2e/run-all.sh
```
