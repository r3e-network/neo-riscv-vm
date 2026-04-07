//! Runtime stack value types for C#-compiled smart contracts.
//!
//! Provides a `StackValue` enum matching NeoVM semantics, with conversions
//! to and from the ABI-level `StackValue` used at the host boundary.

use alloc::vec::Vec;
use neo_riscv_abi::StackValue as AbiStackValue;

// Tag constants matching NeoVM type codes.
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

/// Runtime stack value used inside the compiled contract.
///
/// This mirrors the ABI `StackValue` but adds the `Buffer` variant
/// (which is converted to `ByteString` when crossing the ABI boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackValue {
    /// 64-bit signed integer.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// Byte string (immutable bytes).
    ByteString(Vec<u8>),
    /// Arbitrary-precision integer (little-endian two's complement).
    BigInteger(Vec<u8>),
    /// Array of stack values.
    Array(Vec<StackValue>),
    /// Struct (ordered collection, deep-copied on dup).
    Struct(Vec<StackValue>),
    /// Map (key-value pairs).
    Map(Vec<(StackValue, StackValue)>),
    /// Null value.
    Null,
    /// Interop object handle.
    Interop(u64),
    /// Iterator handle.
    Iterator(u64),
    /// Mutable byte buffer.
    Buffer(Vec<u8>),
    /// Pointer (instruction offset, internal use).
    Pointer(i64),
}

impl StackValue {
    /// Returns the type tag for this value.
    #[must_use]
    pub fn type_tag(&self) -> u8 {
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

    /// Converts this runtime value to an ABI stack value.
    ///
    /// `Buffer` is mapped to `ByteString` since the ABI does not have a
    /// separate buffer variant.
    #[must_use]
    pub fn to_abi(&self) -> AbiStackValue {
        match self {
            StackValue::Integer(v) => AbiStackValue::Integer(*v),
            StackValue::Boolean(v) => AbiStackValue::Boolean(*v),
            StackValue::ByteString(v) => AbiStackValue::ByteString(v.clone()),
            StackValue::BigInteger(v) => AbiStackValue::BigInteger(v.clone()),
            StackValue::Array(items) => {
                AbiStackValue::Array(items.iter().map(StackValue::to_abi).collect())
            }
            StackValue::Struct(items) => {
                AbiStackValue::Struct(items.iter().map(StackValue::to_abi).collect())
            }
            StackValue::Map(pairs) => AbiStackValue::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (k.to_abi(), v.to_abi()))
                    .collect(),
            ),
            StackValue::Null => AbiStackValue::Null,
            StackValue::Interop(h) => AbiStackValue::Interop(*h),
            StackValue::Iterator(h) => AbiStackValue::Iterator(*h),
            StackValue::Buffer(v) => AbiStackValue::ByteString(v.clone()),
            StackValue::Pointer(v) => AbiStackValue::Pointer(*v),
        }
    }

    /// Creates a runtime value from an ABI stack value.
    #[must_use]
    pub fn from_abi(abi: &AbiStackValue) -> Self {
        match abi {
            AbiStackValue::Integer(v) => StackValue::Integer(*v),
            AbiStackValue::Boolean(v) => StackValue::Boolean(*v),
            AbiStackValue::ByteString(v) => StackValue::ByteString(v.clone()),
            AbiStackValue::BigInteger(v) => StackValue::BigInteger(v.clone()),
            AbiStackValue::Array(items) => {
                StackValue::Array(items.iter().map(StackValue::from_abi).collect())
            }
            AbiStackValue::Struct(items) => {
                StackValue::Struct(items.iter().map(StackValue::from_abi).collect())
            }
            AbiStackValue::Map(pairs) => StackValue::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (StackValue::from_abi(k), StackValue::from_abi(v)))
                    .collect(),
            ),
            AbiStackValue::Null => StackValue::Null,
            AbiStackValue::Interop(h) => StackValue::Interop(*h),
            AbiStackValue::Iterator(h) => StackValue::Iterator(*h),
            AbiStackValue::Pointer(v) => StackValue::Pointer(*v),
            AbiStackValue::Buffer(v) => StackValue::Buffer(v.clone()),
        }
    }
}
