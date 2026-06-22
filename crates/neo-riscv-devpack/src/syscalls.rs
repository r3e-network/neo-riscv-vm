//! Low-level `System.*` syscall wrappers for the RISC-V devpack SDK.
//!
//! Each function here builds the evaluation-stack payload for a NEO interop
//! (using [`build_contract_call_stack`] or hand-built `Vec<StackValue>`) and
//! invokes it through [`ffi::invoke_host_call`]. Results are decoded into
//! native Rust types where possible. Functions prefixed `try_` propagate host
//! errors via `Result`; the plain variants return a default on host failure.
//!
//! These mirror the `System.*` interop surface of upstream NeoVM so that
//! contracts ported from C# `Neo.SmartContract.Framework` map directly.

use crate::{api_ids, ffi};
use alloc::{string::String, vec, vec::Vec};
use neo_riscv_abi::StackValue;

/// Call-flags bitmask allowing all operations (read/write state, call, notify).
pub const CALL_FLAGS_ALL: u8 = 0x0f;

// System.Contract

/// Call a method on another contract with all call flags (`System.Contract.Call`).
///
/// Returns the method's return value, or [`StackValue::Null`] on host failure.
pub fn contract_call(hash: &[u8], method: &str, args: &[StackValue]) -> StackValue {
    contract_call_with_flags(hash, method, CALL_FLAGS_ALL, args)
}

/// Build the evaluation stack for a `System.Contract.Call` host invocation.
///
/// This is the general-purpose version that accepts an arbitrary-length byte
/// slice for the contract hash, suitable for user-deployed contracts whose hash
/// may originate from external input.  For native-contract calls where the hash
/// is always a fixed 20-byte `UInt160`, prefer
/// [`crate::native::build_contract_call_stack`] which enforces the length at
/// compile time via `&[u8; 20]`.
#[must_use]
pub fn build_contract_call_stack(
    hash: &[u8],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> Vec<StackValue> {
    vec![
        StackValue::Array(args.to_vec()),
        StackValue::Integer(i64::from(call_flags)),
        StackValue::ByteString(method.as_bytes().to_vec()),
        StackValue::ByteString(hash.to_vec()),
    ]
}

/// Call a method on another contract with explicit call flags
/// (`System.Contract.Call`).
///
/// Returns the method's return value, or [`StackValue::Null`] on host failure.
pub fn contract_call_with_flags(
    hash: &[u8],
    method: &str,
    call_flags: u8,
    args: &[StackValue],
) -> StackValue {
    let stack = build_contract_call_stack(hash, method, call_flags, args);
    ffi::invoke_host_call(api_ids::CONTRACT_CALL, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Deploy a new contract (`System.Contract.Create`).
///
/// Returns the deployed contract's hash, or [`StackValue::Null`] on host failure.
pub fn contract_create(nef: &[u8], manifest: &[u8]) -> StackValue {
    let stack = vec![
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::CONTRACT_CREATE, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Update an existing contract's script/manifest (`System.Contract.Update`).
pub fn contract_update(nef: &[u8], manifest: &[u8]) {
    let stack = vec![
        StackValue::ByteString(nef.to_vec()),
        StackValue::ByteString(manifest.to_vec()),
    ];
    let _ = ffi::invoke_host_call(api_ids::CONTRACT_UPDATE, &stack);
}

/// Get the call flags of the current invocation
/// (`System.Contract.GetCallFlags`).
pub fn contract_get_call_flags() -> StackValue {
    ffi::invoke_host_call(api_ids::CONTRACT_GET_CALL_FLAGS, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Derive a standard account from a public key
/// (`System.Contract.CreateStandardAccount`).
pub fn contract_create_standard_account(pubkey: &[u8]) -> StackValue {
    let stack = vec![StackValue::ByteString(pubkey.to_vec())];
    ffi::invoke_host_call(api_ids::CONTRACT_CREATE_STANDARD_ACCOUNT, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Derive an m-of-n multisig account from public keys
/// (`System.Contract.CreateMultisigAccount`).
///
/// `m` is the signature threshold; `pubkeys` is the full set of public keys.
pub fn contract_create_multisig_account(m: i64, pubkeys: &[Vec<u8>]) -> StackValue {
    let keys: Vec<StackValue> = pubkeys
        .iter()
        .map(|k| StackValue::ByteString(k.clone()))
        .collect();
    let stack = vec![StackValue::Array(keys), StackValue::Integer(m)];
    ffi::invoke_host_call(api_ids::CONTRACT_CREATE_MULTISIG_ACCOUNT, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Run the native on-persist hook (`System.Contract.NativeOnPersist`).
///
/// Invoked by the ledger for native contracts at the start of block persistence.
pub fn contract_native_on_persist() {
    let _ = ffi::invoke_host_call(api_ids::CONTRACT_NATIVE_ON_PERSIST, &[]);
}

/// Run the native post-persist hook (`System.Contract.NativePostPersist`).
///
/// Invoked by the ledger for native contracts at the end of block persistence.
pub fn contract_native_post_persist() {
    let _ = ffi::invoke_host_call(api_ids::CONTRACT_NATIVE_POST_PERSIST, &[]);
}

// System.Runtime

/// Emit a `Runtime.Notify` event with the given name and state items.
pub fn runtime_notify(event: &str, state: &[StackValue]) {
    // The host's Runtime.Notify expects exactly [eventName, Array(state)] — the state
    // items must be wrapped in a single Array, not flattened onto the stack.
    let stack = vec![
        StackValue::ByteString(event.as_bytes().to_vec()),
        StackValue::Array(state.to_vec()),
    ];
    let _ = ffi::invoke_host_call(api_ids::RUNTIME_NOTIFY, &stack);
}

/// Emit a `Runtime.Log` message.
pub fn runtime_log(message: &str) {
    let stack = vec![StackValue::ByteString(message.as_bytes().to_vec())];
    let _ = ffi::invoke_host_call(api_ids::RUNTIME_LOG, &stack);
}

/// Check whether `hash` has signed the current transaction
/// (`System.Runtime.CheckWitness`). Returns `false` on host failure.
pub fn runtime_check_witness(hash: &[u8]) -> bool {
    let stack = vec![StackValue::ByteString(hash.to_vec())];
    ffi::invoke_host_call(api_ids::RUNTIME_CHECK_WITNESS, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}

/// Witness check with explicit error propagation.
/// Returns `Ok(true/false)` on success, `Err(...)` if the host call fails.
/// Prefer this over `runtime_check_witness` for authorization decisions.
pub fn try_runtime_check_witness(hash: &[u8]) -> Result<bool, String> {
    let stack = vec![StackValue::ByteString(hash.to_vec())];
    let result = ffi::invoke_host_call(api_ids::RUNTIME_CHECK_WITNESS, &stack)?;
    match result.first() {
        Some(StackValue::Boolean(b)) => Ok(*b),
        _ => Ok(false),
    }
}

/// Get notifications emitted by `hash` (`System.Runtime.GetNotifications`).
///
/// Returns the notification array, or an empty vec on host failure.
pub fn runtime_get_notifications(hash: &[u8]) -> Vec<StackValue> {
    let stack = vec![StackValue::ByteString(hash.to_vec())];
    ffi::invoke_host_call(api_ids::RUNTIME_GET_NOTIFICATIONS, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Array(items) | StackValue::Struct(items) => Some(items),
            _ => None,
        })
        .unwrap_or_default()
}

/// Explicitly burn `gas` units of gas (`System.Runtime.BurnGas`).
pub fn runtime_burn_gas(gas: i64) {
    let stack = vec![StackValue::Integer(gas)];
    let _ = ffi::invoke_host_call(api_ids::RUNTIME_BURN_GAS, &stack);
}

/// Dynamically load and execute `script` (`System.Runtime.LoadScript`).
///
/// Returns the loaded script's return value, or [`StackValue::Null`] on failure.
pub fn runtime_load_script(script: &[u8], call_flags: u8, args: &[StackValue]) -> StackValue {
    let stack = vec![
        StackValue::Array(args.to_vec()),
        StackValue::Integer(i64::from(call_flags)),
        StackValue::ByteString(script.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::RUNTIME_LOAD_SCRIPT, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

// Zero-arg Runtime getters

/// Return the platform identifier string (`System.Runtime.Platform`, e.g. "NEO").
pub fn runtime_platform() -> String {
    ffi::invoke_host_call(api_ids::RUNTIME_PLATFORM, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::ByteString(b) => String::from_utf8(b).ok(),
            _ => None,
        })
        .unwrap_or_default()
}

/// Return the current trigger type (`System.Runtime.GetTrigger`).
pub fn runtime_get_trigger() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_TRIGGER, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the network magic number (`System.Runtime.GetNetwork`).
pub fn runtime_get_network() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_NETWORK, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the address version byte (`System.Runtime.GetAddressVersion`).
pub fn runtime_get_address_version() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_ADDRESS_VERSION, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the current transaction/container (`System.Runtime.GetScriptContainer`).
pub fn runtime_get_script_container() -> StackValue {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_SCRIPT_CONTAINER, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Return the hash of the currently executing contract
/// (`System.Runtime.GetExecutingScriptHash`).
pub fn runtime_get_executing_script_hash() -> Vec<u8> {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_EXECUTING_SCRIPT_HASH, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::ByteString(b) => Some(b),
            _ => None,
        })
        .unwrap_or_default()
}

/// Return the hash of the calling contract
/// (`System.Runtime.GetCallingScriptHash`).
pub fn runtime_get_calling_script_hash() -> Vec<u8> {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_CALLING_SCRIPT_HASH, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::ByteString(b) => Some(b),
            _ => None,
        })
        .unwrap_or_default()
}

/// Return the hash of the entry-point contract
/// (`System.Runtime.GetEntryScriptHash`).
pub fn runtime_get_entry_script_hash() -> Vec<u8> {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_ENTRY_SCRIPT_HASH, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::ByteString(b) => Some(b),
            _ => None,
        })
        .unwrap_or_default()
}

/// Return the current block timestamp (`System.Runtime.GetTime`).
pub fn runtime_get_time() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_TIME, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the current call-chain depth (`System.Runtime.GetInvocationCounter`).
pub fn runtime_get_invocation_counter() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_INVOCATION_COUNTER, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the remaining gas for the current invocation
/// (`System.Runtime.GasLeft`).
pub fn runtime_gas_left() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GAS_LEFT, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return a deterministic per-block random number
/// (`System.Runtime.GetRandom`).
pub fn runtime_get_random() -> i64 {
    ffi::invoke_host_call(api_ids::RUNTIME_GET_RANDOM, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Integer(i) => Some(i),
            _ => None,
        })
        .unwrap_or(0)
}

/// Return the current transaction's signers
/// (`System.Runtime.CurrentSigners`).
pub fn runtime_current_signers() -> StackValue {
    ffi::invoke_host_call(api_ids::RUNTIME_CURRENT_SIGNERS, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

// System.Crypto

/// Verify a single signature (`System.Crypto.CheckSig`). Returns `false` on
/// host failure.
pub fn crypto_verify_signature(message: &[u8], pubkey: &[u8], signature: &[u8]) -> bool {
    let stack = vec![
        StackValue::ByteString(message.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
        StackValue::ByteString(signature.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::CRYPTO_VERIFY_SIGNATURE, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}

/// Signature verification with explicit error propagation.
/// Returns `Ok(true/false)` on success, `Err(...)` if the host call fails.
pub fn try_crypto_verify_signature(
    message: &[u8],
    pubkey: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    let stack = vec![
        StackValue::ByteString(message.to_vec()),
        StackValue::ByteString(pubkey.to_vec()),
        StackValue::ByteString(signature.to_vec()),
    ];
    let result = ffi::invoke_host_call(api_ids::CRYPTO_VERIFY_SIGNATURE, &stack)?;
    match result.first() {
        Some(StackValue::Boolean(b)) => Ok(*b),
        _ => Ok(false),
    }
}

/// Verify an m-of-n multisignature (`System.Crypto.CheckMultisig`). Returns
/// `false` on host failure.
pub fn crypto_check_multisig(message: &[u8], pubkeys: &[Vec<u8>], signatures: &[Vec<u8>]) -> bool {
    let pk: Vec<StackValue> = pubkeys
        .iter()
        .map(|k| StackValue::ByteString(k.clone()))
        .collect();
    let sig: Vec<StackValue> = signatures
        .iter()
        .map(|s| StackValue::ByteString(s.clone()))
        .collect();
    let stack = vec![
        StackValue::ByteString(message.to_vec()),
        StackValue::Array(pk),
        StackValue::Array(sig),
    ];
    ffi::invoke_host_call(api_ids::CRYPTO_CHECK_MULTISIG, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}

/// Multisig verification with explicit error propagation.
pub fn try_crypto_check_multisig(
    message: &[u8],
    pubkeys: &[Vec<u8>],
    signatures: &[Vec<u8>],
) -> Result<bool, String> {
    let pk: Vec<StackValue> = pubkeys
        .iter()
        .map(|k| StackValue::ByteString(k.clone()))
        .collect();
    let sig: Vec<StackValue> = signatures
        .iter()
        .map(|s| StackValue::ByteString(s.clone()))
        .collect();
    let stack = vec![
        StackValue::ByteString(message.to_vec()),
        StackValue::Array(pk),
        StackValue::Array(sig),
    ];
    let result = ffi::invoke_host_call(api_ids::CRYPTO_CHECK_MULTISIG, &stack)?;
    match result.first() {
        Some(StackValue::Boolean(b)) => Ok(*b),
        _ => Ok(false),
    }
}

// System.Iterator

/// Advance an iterator to the next element (`System.Iterator.Next`).
///
/// Returns `true` if a next element exists, `false` if exhausted or on failure.
pub fn iterator_next(iterator_handle: u64) -> bool {
    let stack = vec![StackValue::Iterator(iterator_handle)];
    ffi::invoke_host_call(api_ids::ITERATOR_NEXT, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .and_then(|v| match v {
            StackValue::Boolean(b) => Some(b),
            _ => None,
        })
        .unwrap_or(false)
}

/// Get the current element of an iterator (`System.Iterator.Value`).
///
/// Returns the current value, or [`StackValue::Null`] on failure.
pub fn iterator_value(iterator_handle: u64) -> StackValue {
    let stack = vec![StackValue::Iterator(iterator_handle)];
    ffi::invoke_host_call(api_ids::ITERATOR_VALUE, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}
