//! Neo RISC-V ABI types and utilities.
//!
//! This crate defines the Application Binary Interface (ABI) between the RISC-V guest VM
//! and the host runtime. It provides serializable types for VM execution results, stack values,
//! and host callback communication.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod callback_codec;
pub mod fast_codec;

/// VM execution state after script completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    /// Execution completed successfully.
    Halt,
    /// Execution failed with an error.
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendKind {
    Interpreter,
}

/// NeoVM stack value types.
///
/// Represents all possible types that can exist on the NeoVM evaluation stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackValue {
    /// 64-bit signed integer.
    Integer(i64),
    /// Arbitrary-precision integer (little-endian bytes).
    BigInteger(Vec<u8>),
    /// Byte array.
    ByteString(Vec<u8>),
    /// Boolean value.
    Boolean(bool),
    /// Array of stack values.
    Array(Vec<StackValue>),
    /// Struct (ordered collection).
    Struct(Vec<StackValue>),
    /// Map (key-value pairs).
    Map(Vec<(StackValue, StackValue)>),
    /// Interop object handle.
    Interop(u64),
    /// Iterator handle.
    Iterator(u64),
    /// Null value.
    Null,
    /// Pointer (for internal use).
    Pointer(i64),
}

/// Result of VM script execution.
///
/// Contains the final execution state, stack contents, gas consumed, and optional fault message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Gas consumed in pico units.
    pub fee_consumed_pico: i64,
    /// Final execution state (Halt or Fault).
    pub state: VmState,
    /// Final evaluation stack contents.
    pub stack: Vec<StackValue>,
    /// Optional fault message for FAULT states (e.g., ABORTMSG, ASSERTMSG).
    /// When present, this is the user-facing error string without internal trace.
    #[serde(default)]
    pub fault_message: Option<String>,
}

#[must_use]
pub fn interop_hash(name: &str) -> u32 {
    let digest = Sha256::digest(name.as_bytes());
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}
