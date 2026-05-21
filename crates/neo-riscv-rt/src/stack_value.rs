//! Runtime stack value helpers for C#-compiled smart contracts.
//!
//! The runtime uses the canonical ABI StackValue from `neo-vm-rs` through
//! `neo-riscv-abi`. This avoids maintaining a second stack value enum in the
//! PolkaVM runtime layer. Compact runtime tags are also re-exported from the
//! shared VM crate so generated code and the interpreter agree on classification.

pub use neo_riscv_abi::{
    byte_sequence_bytes, byte_sequence_len, concat_byte_sequences, default_value_for_type_tag,
    new_array_default_value_for_type_tag, normalize_stack_item_type_tag, slice_byte_sequence,
    StackValue, TAG_ARRAY, TAG_BIG_INTEGER, TAG_BOOLEAN, TAG_BUFFER, TAG_BYTESTRING, TAG_INTEGER,
    TAG_INTEROP, TAG_ITERATOR, TAG_MAP, TAG_NULL, TAG_POINTER, TAG_STRUCT,
};
