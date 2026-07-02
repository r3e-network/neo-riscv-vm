#![no_std]
#![deny(missing_docs)]

//! NEO smart-contract developer SDK for the RISC-V (PolkaVM) execution backend.
//!
//! This crate is the in-guest equivalent of `Neo.SmartContract.Framework` for
//! C# NeoVM contracts. Contracts compiled to RISC-V link against it to invoke
//! NEO runtime services — storage, runtime introspection, native-contract
//! calls, cryptography, and events — via the PolkaVM `host_call` syscall.
//!
//! # Layout
//!
//! - [`syscalls`] — low-level syscall wrappers that build the `System.*` interop
//!   stack and invoke the host.
//! - [`runtime`] — re-export of the `runtime_*` syscall family (gas, witnesses,
//!   script hashes, notifications, etc.).
//! - [`storage`] — key/value persistent storage (`get`/`put`/`delete`).
//! - [`native`] — typed bindings to NEO native contracts (NEO/Gas/Policy/Ledger
//!   /Oracle/Notary/RoleManagement/StdLib/Treasury/Crypto/ContractManagement).
//! - [`events`] — `Runtime.Notify` event emission helpers.
//! - [`signing`] — signature verification (`check_witness`, `verify_signature`,
//!   `verify_multisig`).
//! - [`codec`] / [`parser`] — encode/decode and parse results to/from
//!   [`StackValue`]s.
//! - [`api_ids`] — numeric ids identifying each `System.*` interop.
//! - [`ffi`] — the raw `host_call` FFI shim.
//!
//! [`StackValue`]: neo_riscv_abi::StackValue

extern crate alloc;

/// Numeric syscall (interop) ids — one per `System.*` service the host exposes.
pub mod api_ids;
/// Encode/decode helpers for syscall request/response payloads.
pub mod codec;
/// `Runtime.Notify` event-emission helpers.
pub mod events;
/// Raw `host_call` FFI shim used by the syscall wrappers.
pub mod ffi;
/// Typed bindings to NEO native contracts.
pub mod native;
/// Parse and format [`StackValue`] results into native Rust types.
///
/// [`StackValue`]: neo_riscv_abi::StackValue
pub mod parser;
/// Signature and witness verification (`check_witness`, `verify_signature`, `verify_multisig`).
pub mod signing;
/// Persistent key/value contract storage.
pub mod storage;
/// Low-level `System.*` syscall wrappers (contract calls, runtime, etc.).
pub mod syscalls;

/// Re-exported codec encode/decode helpers for fast-codec payloads.
pub use codec::{
    decode_bool_result, decode_bytes_result, decode_int_result, decode_string_result, encode_bytes,
    encode_int_params, encode_string_params,
};
/// Re-exported `Runtime.Notify` event helpers.
pub use events::{
    notify, notify_args, notify_int, notify_key_value, notify_string, notify_two_strings,
};
/// Re-exported result-parsing helpers.
pub use parser::{format_stack_value, parse_int_result, parse_stack_value, parse_string_result};
/// Re-exported signing helpers.
pub use signing::{check_witness, verify_multisig, verify_signature};
/// Re-exported storage operations.
pub use storage::{delete, get, put};
/// Re-exported contract-management syscalls.
pub use syscalls::{contract_call, contract_create, contract_update};

/// Runtime-introspection syscall family (gas, witnesses, script hashes,
/// notifications, time, random, etc.).
///
/// These are grouped under `runtime` to mirror the `Neo.SmartContract.Framework`
/// `Runtime` static class that NeoVM contract authors are familiar with.
pub mod runtime {
    /// Re-exported `System.Runtime.*` syscall wrappers.
    pub use crate::syscalls::{
        runtime_burn_gas, runtime_check_witness, runtime_current_signers, runtime_gas_left,
        runtime_get_address_version, runtime_get_calling_script_hash,
        runtime_get_entry_script_hash, runtime_get_executing_script_hash,
        runtime_get_invocation_counter, runtime_get_network, runtime_get_notifications,
        runtime_get_random, runtime_get_script_container, runtime_get_time, runtime_load_script,
        runtime_log, runtime_notify, runtime_platform,
    };
}
