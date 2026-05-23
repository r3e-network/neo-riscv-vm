//! Neo RISC-V ABI types and utilities.
//!
//! This crate defines the Application Binary Interface (ABI) between the RISC-V guest VM
//! and the host runtime. It provides serializable types for VM execution results, stack values,
//! and host callback communication.

#![no_std]

extern crate alloc;

pub mod callback_codec;
pub mod fast_codec;
pub mod result_codec;

pub use neo_vm_rs::{
    byte_sequence_bytes, byte_sequence_len, concat_splice_values, default_value_for_type_tag,
    encode_integer, interop_hash, new_array_default_value_for_type_tag,
    normalize_stack_item_type_tag, slice_splice_value, stack_value_as_bool, stack_value_as_bytes,
    stack_value_as_fixed_bytes, stack_value_as_i64, stack_value_as_string, stack_value_as_u32,
    stack_value_as_u8, stack_value_into_items, stack_value_span_bytes, syscall_arg_count,
    BackendKind, ExecutionResult, OpCode, StackValue, VmContext, VmState,
    COMPACT_TAG_ARRAY as TAG_ARRAY, COMPACT_TAG_BIG_INTEGER as TAG_BIG_INTEGER,
    COMPACT_TAG_BOOLEAN as TAG_BOOLEAN, COMPACT_TAG_BUFFER as TAG_BUFFER,
    COMPACT_TAG_BYTESTRING as TAG_BYTESTRING, COMPACT_TAG_INTEGER as TAG_INTEGER,
    COMPACT_TAG_INTEROP as TAG_INTEROP, COMPACT_TAG_ITERATOR as TAG_ITERATOR,
    COMPACT_TAG_MAP as TAG_MAP, COMPACT_TAG_NULL as TAG_NULL,
    COMPACT_TAG_POINTER as TAG_POINTER, COMPACT_TAG_STRUCT as TAG_STRUCT, STACK_VALUE_CODEC_TAG_ARRAY,
    STACK_VALUE_CODEC_TAG_BIG_INTEGER, STACK_VALUE_CODEC_TAG_BOOLEAN, STACK_VALUE_CODEC_TAG_BUFFER,
    STACK_VALUE_CODEC_TAG_BYTESTRING, STACK_VALUE_CODEC_TAG_INTEGER, STACK_VALUE_CODEC_TAG_INTEROP,
    STACK_VALUE_CODEC_TAG_ITERATOR, STACK_VALUE_CODEC_TAG_MAP, STACK_VALUE_CODEC_TAG_NULL,
    STACK_VALUE_CODEC_TAG_POINTER, STACK_VALUE_CODEC_TAG_STRUCT,
};
pub use neo_vm_rs::{runtime, semantics};

#[cfg(test)]
mod tests {
    use super::{interop_hash, syscall_arg_count};

    #[test]
    fn variable_stack_syscalls_keep_the_full_stack() {
        assert_eq!(
            syscall_arg_count(interop_hash("System.Contract.CallNative")),
            usize::MAX,
        );
        assert_eq!(
            syscall_arg_count(interop_hash("System.Crypto.CheckMultisig")),
            usize::MAX,
        );
    }

    #[test]
    fn create_multisig_account_uses_neovm_descriptor_arguments() {
        assert_eq!(
            syscall_arg_count(interop_hash("System.Contract.CreateMultisigAccount")),
            2,
        );
    }
}
