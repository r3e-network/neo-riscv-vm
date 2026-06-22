//! Persistent key/value contract storage (`System.Storage.*` syscalls).
//!
//! Provides `get`/`put`/`delete`/`find` against the current contract's storage
//! context, plus a [`local`] submodule for the contract's private storage.
//! Variants prefixed `try_` propagate host errors via `Result`.

use crate::{api_ids, ffi};
use alloc::{string::String, vec, vec::Vec};
use neo_riscv_abi::StackValue;

/// Get the current contract's storage context (`System.Storage.GetContext`).
pub fn get_context() -> StackValue {
    ffi::invoke_host_call(api_ids::STORAGE_GET_CONTEXT, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Get a read-only storage context (`System.Storage.GetReadOnlyContext`).
pub fn get_readonly_context() -> StackValue {
    ffi::invoke_host_call(api_ids::STORAGE_GET_READONLY_CONTEXT, &[])
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Convert a storage context to read-only (`System.Storage.AsReadOnly`).
pub fn as_read_only(context: &StackValue) -> StackValue {
    let stack = vec![context.clone()];
    ffi::invoke_host_call(api_ids::STORAGE_AS_READ_ONLY, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Get a value from storage by key.
///
/// Returns `Some(value)` if found, `None` if not found, silently returns `None` on host error.
/// Prefer `try_get` to distinguish "key not found" from "host error".
pub fn get(key: &[u8]) -> Option<Vec<u8>> {
    let stack = vec![get_readonly_context(), StackValue::ByteString(key.to_vec())];
    let result = ffi::invoke_host_call(api_ids::STORAGE_GET, &stack).ok()?;
    match result.first()? {
        StackValue::ByteString(data) => Some(data.clone()),
        _ => None,
    }
}

/// Get a value from storage with explicit error propagation.
///
/// Returns `Ok(Some(value))` on success, `Ok(None)` if key not found,
/// `Err(...)` if the host call itself fails.
pub fn try_get(key: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let stack = vec![get_readonly_context(), StackValue::ByteString(key.to_vec())];
    let result = ffi::invoke_host_call(api_ids::STORAGE_GET, &stack)?;
    match result.first() {
        Some(StackValue::ByteString(data)) => Ok(Some(data.clone())),
        _ => Ok(None),
    }
}

/// Write a key/value pair to storage (`System.Storage.Put`).
pub fn put(key: &[u8], value: &[u8]) {
    let stack = vec![
        get_context(),
        StackValue::ByteString(key.to_vec()),
        StackValue::ByteString(value.to_vec()),
    ];
    let _ = ffi::invoke_host_call(api_ids::STORAGE_PUT, &stack);
}

/// Write a value to storage with explicit error propagation.
/// Returns `Ok(())` on success, `Err(...)` if the host call fails.
pub fn try_put(key: &[u8], value: &[u8]) -> Result<(), String> {
    let stack = vec![
        get_context(),
        StackValue::ByteString(key.to_vec()),
        StackValue::ByteString(value.to_vec()),
    ];
    ffi::invoke_host_call(api_ids::STORAGE_PUT, &stack).map(|_| ())
}

/// Delete a key from storage (`System.Storage.Delete`).
pub fn delete(key: &[u8]) {
    let stack = vec![get_context(), StackValue::ByteString(key.to_vec())];
    let _ = ffi::invoke_host_call(api_ids::STORAGE_DELETE, &stack);
}

/// Delete a key from storage with explicit error propagation.
pub fn try_delete(key: &[u8]) -> Result<(), String> {
    let stack = vec![get_context(), StackValue::ByteString(key.to_vec())];
    ffi::invoke_host_call(api_ids::STORAGE_DELETE, &stack).map(|_| ())
}

/// Seek over keys matching `prefix` (`System.Storage.Find`).
///
/// `options` is a `FindOptions` bitmask controlling key/value/prefix behavior.
/// Returns an iterator handle, or [`StackValue::Null`] on failure.
pub fn find(prefix: &[u8], options: i64) -> StackValue {
    let stack = vec![
        get_readonly_context(),
        StackValue::ByteString(prefix.to_vec()),
        StackValue::Integer(options),
    ];
    ffi::invoke_host_call(api_ids::STORAGE_FIND, &stack)
        .ok()
        .and_then(|r| r.into_iter().next())
        .unwrap_or(StackValue::Null)
}

/// Storage operations scoped to the current contract's private storage
/// (`System.Storage.Local.*`).
pub mod local {
    use super::*;

    /// Get a value from private storage by key.
    pub fn get(key: &[u8]) -> Option<Vec<u8>> {
        let stack = vec![StackValue::ByteString(key.to_vec())];
        let result = ffi::invoke_host_call(api_ids::STORAGE_LOCAL_GET, &stack).ok()?;
        match result.first()? {
            StackValue::ByteString(data) => Some(data.clone()),
            _ => None,
        }
    }

    /// Get a value from private storage with explicit error propagation.
    pub fn try_get(key: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let stack = vec![StackValue::ByteString(key.to_vec())];
        let result = ffi::invoke_host_call(api_ids::STORAGE_LOCAL_GET, &stack)?;
        match result.first() {
            Some(StackValue::ByteString(data)) => Ok(Some(data.clone())),
            _ => Ok(None),
        }
    }

    /// Write a key/value pair to private storage.
    pub fn put(key: &[u8], value: &[u8]) {
        let stack = vec![
            StackValue::ByteString(key.to_vec()),
            StackValue::ByteString(value.to_vec()),
        ];
        let _ = ffi::invoke_host_call(api_ids::STORAGE_LOCAL_PUT, &stack);
    }

    /// Write a value to private storage with explicit error propagation.
    pub fn try_put(key: &[u8], value: &[u8]) -> Result<(), String> {
        let stack = vec![
            StackValue::ByteString(key.to_vec()),
            StackValue::ByteString(value.to_vec()),
        ];
        ffi::invoke_host_call(api_ids::STORAGE_LOCAL_PUT, &stack).map(|_| ())
    }

    /// Delete a key from private storage.
    pub fn delete(key: &[u8]) {
        let stack = vec![StackValue::ByteString(key.to_vec())];
        let _ = ffi::invoke_host_call(api_ids::STORAGE_LOCAL_DELETE, &stack);
    }

    /// Delete a key from private storage with explicit error propagation.
    pub fn try_delete(key: &[u8]) -> Result<(), String> {
        let stack = vec![StackValue::ByteString(key.to_vec())];
        ffi::invoke_host_call(api_ids::STORAGE_LOCAL_DELETE, &stack).map(|_| ())
    }

    /// Seek over keys matching `prefix` in private storage.
    pub fn find(prefix: &[u8], options: i64) -> StackValue {
        let stack = vec![
            StackValue::ByteString(prefix.to_vec()),
            StackValue::Integer(options),
        ];
        ffi::invoke_host_call(api_ids::STORAGE_LOCAL_FIND, &stack)
            .ok()
            .and_then(|r| r.into_iter().next())
            .unwrap_or(StackValue::Null)
    }
}
