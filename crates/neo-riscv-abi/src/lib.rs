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
pub mod result_codec;

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
    /// Byte array (immutable).
    ByteString(Vec<u8>),
    /// Mutable byte buffer (distinct from ByteString in NeoVM).
    Buffer(Vec<u8>),
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
    /// Instruction pointer (NEF script offset) at the moment a FAULT was raised.
    /// `None` for HALT or when the faulting IP cannot be attributed. Used by the
    /// C# adapter to populate `ExecutionContext.InstructionPointer` on fault so
    /// that dev-time test harnesses asserting exact fault offsets (e.g., Test_Abort)
    /// see the real opcode offset instead of 0. `#[serde(default)]` keeps the
    /// codec wire-compatible with older guest/host pairs.
    #[serde(default)]
    pub fault_ip: Option<u32>,
    /// Local variables of the faulting frame, serialized via `fast_codec`. `None` for
    /// HALT or when the guest did not capture a locals snapshot. Used by the C# adapter
    /// to populate `ExecutionContext.LocalVariables` on fault so dev-time test harnesses
    /// (Test_Abort, Test_Assert*) that introspect local values see the runtime state
    /// instead of `null`. `#[serde(default)]` keeps the codec wire-compatible.
    #[serde(default)]
    pub fault_locals: Option<Vec<u8>>,
}

/// Returns the number of stack arguments consumed by a NeoVM syscall.
///
/// For known syscalls, returns the exact argument count so the guest can pass
/// only the needed items instead of the entire evaluation stack. This avoids
/// encoding/decoding thousands of items through the host boundary when only
/// a few are needed (e.g., `System.Contract.Call` needs 4 items, not 1,230).
///
/// Returns `usize::MAX` for unknown syscalls (full stack passthrough).
#[must_use]
pub fn syscall_arg_count(api: u32) -> usize {
    match api {
        // System.Contract
        0x525b_7d62 => 4,          // System.Contract.Call
        0x852c_35ce => 2,          // System.Contract.Create
        0x1d33_c631 => 2,          // System.Contract.Update
        0x93bc_db2e => 0,          // System.Contract.NativeOnPersist
        0x165d_a144 => 0,          // System.Contract.NativePostPersist
        0x813a_da95 => 0,          // System.Contract.GetCallFlags
        0x0287_99cf => 1,          // System.Contract.CreateStandardAccount
        0x09e9_336a => usize::MAX, // System.Contract.CreateMultisigAccount (count-based suffix)
        // System.Runtime (with args)
        0x8cec_27f8 => 1, // System.Runtime.CheckWitness
        0x616f_0195 => 2, // System.Runtime.Notify
        0x9647_e7cf => 1, // System.Runtime.Log
        0xf135_4327 => 1, // System.Runtime.GetNotifications
        0xbc8c_5ac3 => 1, // System.Runtime.BurnGas
        0x8f80_0cb3 => 3, // System.Runtime.LoadScript
        // System.Runtime (0-arg)
        0x0388_c3b7 | 0xf6fc_79b2 | 0xa038_7de9 | 0xe0a0_fbc5 | 0xdc92_494c | 0x3008_512d
        | 0x74a8_fedb | 0x3c6e_5339 | 0x38e2_b4f9 | 0x28a9_de6b | 0xced8_8814 | 0x4311_2784
        | 0x8b18_f1ac => 0,
        // System.Storage
        0xce67_f69b => 0, // System.Storage.GetContext
        0xe26b_b4f6 => 0, // System.Storage.GetReadOnlyContext
        0xe9bf_4c76 => 1, // System.Storage.AsReadOnly
        0xe85e_8dd5 => 1, // System.Storage.Local.Get
        0x0ae3_0c39 => 2, // System.Storage.Local.Put
        0x94f5_5475 => 1, // System.Storage.Local.Delete
        0xf352_7607 => 2, // System.Storage.Local.Find
        0x31e8_5d92 => 2, // System.Storage.Get
        0x9ab8_30df => 3, // System.Storage.Find
        0x8418_3fe6 => 3, // System.Storage.Put
        0xedc5_582f => 2, // System.Storage.Delete
        // System.Crypto
        0x27b3_e756 => 2,          // System.Crypto.CheckSig
        0x3adc_d09e => usize::MAX, // System.Crypto.CheckMultisig (count-based suffix)
        // System.Iterator
        0x9ced_089c => 2, // System.Iterator.Next
        0x1dbf_54f3 => 1, // System.Iterator.Value
        // Unknown: pass full stack for safety
        _ => usize::MAX,
    }
}

#[must_use]
pub fn interop_hash(name: &str) -> u32 {
    let digest = Sha256::digest(name.as_bytes());
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

#[cfg(test)]
mod tests {
    use super::{interop_hash, syscall_arg_count};

    #[test]
    fn count_based_syscalls_keep_the_full_stack() {
        assert_eq!(
            syscall_arg_count(interop_hash("System.Contract.CreateMultisigAccount")),
            usize::MAX,
        );
        assert_eq!(
            syscall_arg_count(interop_hash("System.Crypto.CheckMultisig")),
            usize::MAX,
        );
    }
}
