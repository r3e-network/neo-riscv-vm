//! Runtime stack value helpers for C#-compiled smart contracts.
//!
//! The runtime uses the canonical ABI StackValue from `neo-vm-rs` through
//! `neo-riscv-abi`. This avoids maintaining a second stack value enum in the
//! PolkaVM runtime layer.

pub use neo_riscv_abi::StackValue;

// Tag constants matching NeoVM type codes used by generated code.
pub const TAG_INTEGER: u8 = 0;
pub const TAG_BOOLEAN: u8 = 1;
pub const TAG_BYTESTRING: u8 = 2;
pub const TAG_BIG_INTEGER: u8 = 3;
pub const TAG_ARRAY: u8 = 4;
pub const TAG_STRUCT: u8 = 5;
pub const TAG_MAP: u8 = 6;
pub const TAG_NULL: u8 = 7;
pub const TAG_INTEROP: u8 = 8;
pub const TAG_ITERATOR: u8 = 9;
pub const TAG_BUFFER: u8 = 10;
pub const TAG_POINTER: u8 = 11;

/// Runtime-only helpers layered on the shared StackValue type.
pub trait RuntimeStackValueExt {
    /// Returns the NeoVM runtime type tag for this value.
    fn type_tag(&self) -> u8;
}

impl RuntimeStackValueExt for StackValue {
    fn type_tag(&self) -> u8 {
        match self {
            StackValue::Integer(_) => TAG_INTEGER,
            StackValue::Boolean(_) => TAG_BOOLEAN,
            StackValue::ByteString(_) => TAG_BYTESTRING,
            StackValue::BigInteger(_) => TAG_BIG_INTEGER,
            StackValue::Array(_) => TAG_ARRAY,
            StackValue::Struct(_) => TAG_STRUCT,
            StackValue::Map(_) => TAG_MAP,
            StackValue::Null => TAG_NULL,
            StackValue::Interop(_) => TAG_INTEROP,
            StackValue::Iterator(_) => TAG_ITERATOR,
            StackValue::Buffer(_) => TAG_BUFFER,
            StackValue::Pointer(_) => TAG_POINTER,
        }
    }
}
