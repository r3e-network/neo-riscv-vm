#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod callback_codec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    Halt,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendKind {
    Interpreter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackValue {
    Integer(i64),
    BigInteger(Vec<u8>),
    ByteString(Vec<u8>),
    Boolean(bool),
    Array(Vec<StackValue>),
    Struct(Vec<StackValue>),
    Map(Vec<(StackValue, StackValue)>),
    Interop(u64),
    Iterator(u64),
    Null,
    Pointer(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: VmState,
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
