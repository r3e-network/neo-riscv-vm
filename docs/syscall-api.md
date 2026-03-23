# Syscall API Reference

⚠️ **ARCHITECTURE NOTE**: These are Rust bindings that forward into the Neo N3 C# engine via FFI. Syscall semantics, permission checks, and native execution remain defined by C#.

## Rust Devpack Surface

Current syscall-facing helpers exposed by `neo_riscv_devpack`:

```rust
pub use storage::{delete, get, put};
pub use syscalls::{contract_call, contract_create, contract_update};

pub mod runtime {
    pub use crate::syscalls::{runtime_check_witness, runtime_log, runtime_notify};
}
```

`storage::{get, put, delete}` are also re-exported at the crate root as `get`, `put`, and `delete`.

## System.Contract

### contract_call

```rust
pub fn contract_call(hash: &[u8], method: &str, args: &[StackValue]) -> StackValue
pub fn contract_call_with_flags(
    hash: &[u8],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> StackValue
```

High-level Rust helpers for calling another contract. `contract_call()` defaults to `CallFlags::All (0x0f)`, while `contract_call_with_flags()` lets callers select a narrower flag mask explicitly.

Underlying Neo syscall semantics are still:

```text
System.Contract.Call(contract_hash, method, call_flags, args_array)
```

At the stack boundary consumed by the C# bridge, the four logical operands are:

1. `contract_hash` as a 20-byte `ByteString`
2. `method` as a UTF-8 `ByteString`
3. `call_flags` as an integer
4. `args_array` as one NeoVM array containing the method arguments

In other words, `args` are one packed array at the syscall boundary, not a sequence of loose stack items.

Stack order on the Neo evaluation stack immediately before the syscall executes (bottom -> top):

- `args_array: Array`
- `call_flags: Integer`
- `method: ByteString`
- `contract_hash: ByteString(20)`

### System.Contract.Call flags

The C# bridge validates `call_flags` against the standard Neo `CallFlags` mask:

| Flag | Value |
| --- | --- |
| `None` | `0x00` |
| `ReadStates` | `0x01` |
| `WriteStates` | `0x02` |
| `AllowCall` | `0x04` |
| `AllowNotify` | `0x08` |
| `States` | `0x03` |
| `ReadOnly` | `0x0d` |
| `All` | `0x0f` |

Current runtime tests exercise `System.Contract.Call` with `CallFlags.All (0x0f)`. Safe methods can be further reduced by the C# engine before nested execution.

### contract_create

```rust
pub fn contract_create(nef: &[u8], manifest: &[u8]) -> StackValue
```

Deploy a new contract.

### contract_update

```rust
pub fn contract_update(nef: &[u8], manifest: &[u8])
```

Update existing contract code.

## System.Storage

```rust
pub fn get(key: &[u8]) -> Option<Vec<u8>>
pub fn put(key: &[u8], value: &[u8])
pub fn delete(key: &[u8])
```

These are the current Rust names. Older `storage_get`, `storage_put`, and `storage_delete` spellings are stale and should not be used in new code or docs.

## System.Runtime

```rust
pub fn runtime_notify(event: &str, state: &[StackValue])
pub fn runtime_log(message: &str)
pub fn runtime_check_witness(hash: &[u8]) -> bool
```

There is also a separate placeholder module:

```rust
pub mod events {
    pub fn notify(event_name: &str, state: &[u8])
}
```

## System.Crypto

```rust
pub fn crypto_verify_signature(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool
```
