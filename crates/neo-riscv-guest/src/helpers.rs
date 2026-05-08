extern crate alloc;

use crate::runtime_types::{
    structurally_equal, to_abi_stack, to_abi_value, CompoundIds, StackValue,
};
use core::cell::UnsafeCell;

const POST_SYSCALL_STACK_HEADROOM: usize = 8;
const RETAINED_PREFIX_BUF_SIZE: usize = 2 * 1024 * 1024;
const MAX_RETAINED_DECODE_DEPTH: usize = 64;
const MAX_RETAINED_COLLECTION_LEN: usize = 4096;
const MAX_INTEGER_SIZE: usize = 32;

const RETAINED_TAG_INTEGER: u8 = 0x01;
const RETAINED_TAG_BIGINTEGER: u8 = 0x02;
const RETAINED_TAG_BYTESTRING: u8 = 0x03;
const RETAINED_TAG_BOOLEAN: u8 = 0x04;
const RETAINED_TAG_ARRAY: u8 = 0x05;
const RETAINED_TAG_STRUCT: u8 = 0x06;
const RETAINED_TAG_MAP: u8 = 0x07;
const RETAINED_TAG_INTEROP: u8 = 0x08;
const RETAINED_TAG_ITERATOR: u8 = 0x09;
const RETAINED_TAG_NULL: u8 = 0x0A;
const RETAINED_TAG_POINTER: u8 = 0x0B;
const RETAINED_TAG_BUFFER: u8 = 0x0C;

pub(crate) struct RetainedPrefixBuffer(UnsafeCell<[u8; RETAINED_PREFIX_BUF_SIZE]>);

unsafe impl Sync for RetainedPrefixBuffer {}

impl RetainedPrefixBuffer {
    const fn new() -> Self {
        Self(UnsafeCell::new([0; RETAINED_PREFIX_BUF_SIZE]))
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn as_mut_slice(&self) -> &mut [u8] {
        &mut *self.0.get()
    }

    pub(crate) unsafe fn as_slice(&self, len: usize) -> &[u8] {
        &(&*self.0.get())[..len]
    }
}

static RETAINED_STACK_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
pub(crate) static RETAINED_ARGS_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
static RETAINED_LOCALS_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
static RETAINED_STATIC_FIELDS_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
static RETAINED_ALT_STACK_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
static RETAINED_CONSUMED_MUTATIONS_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
#[cfg(target_arch = "riscv32")]
static RETAINED_INITIAL_STACK_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();
#[cfg(target_arch = "riscv32")]
pub(crate) static RETAINED_CALL_STACK_BUF: RetainedPrefixBuffer = RetainedPrefixBuffer::new();

use crate::SyscallProvider;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use num_bigint::BigInt;
use num_traits::Zero;

#[inline]
pub(crate) fn peek_item(stack: &[StackValue]) -> Result<StackValue, String> {
    stack
        .last()
        .cloned()
        .ok_or_else(|| "stack underflow".to_string())
}

#[inline]
pub(crate) fn pop_item(stack: &mut Vec<StackValue>) -> Result<StackValue, String> {
    stack.pop().ok_or_else(|| "stack underflow".to_string())
}

#[inline]
pub(crate) fn pop_integer(stack: &mut Vec<StackValue>) -> Result<i64, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(if value { 1 } else { 0 }),
        Some(StackValue::ByteString(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::BigInteger(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::Buffer(_, bytes)) => decode_signed_le_bytes(&bytes),
        Some(_) => Err("expected integer on stack".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

#[inline]
pub(crate) fn pop_bigint_pair_allowing_null_false(
    stack: &mut Vec<StackValue>,
) -> Result<Option<(BigInt, BigInt)>, String> {
    let right = stack.pop().ok_or_else(|| "stack underflow".to_string())?;
    let left = stack.pop().ok_or_else(|| "stack underflow".to_string())?;

    if matches!(left, StackValue::Null) || matches!(right, StackValue::Null) {
        return Ok(None);
    }

    Ok(Some((
        bigint_for_comparison(left)?,
        bigint_for_comparison(right)?,
    )))
}

fn bigint_for_comparison(value: StackValue) -> Result<BigInt, String> {
    match value {
        StackValue::Integer(value) => Ok(BigInt::from(value)),
        StackValue::BigInteger(value) => decode_signed_le_bytes_bigint(&value),
        StackValue::ByteString(value) => decode_signed_le_bytes_bigint(&value),
        StackValue::Boolean(value) => Ok(BigInt::from(if value { 1 } else { 0 })),
        StackValue::Pointer(_) => Err("expected integer on stack".to_string()),
        StackValue::Array(..) => Err("expected integer on stack".to_string()),
        StackValue::Struct(..) => Err("expected integer on stack".to_string()),
        StackValue::Map(..) => Err("expected integer on stack".to_string()),
        StackValue::Buffer(_, _) => Err("expected integer on stack".to_string()),
        StackValue::Interop(_) => Err("expected integer on stack".to_string()),
        StackValue::Iterator(_) => Err("expected integer on stack".to_string()),
        StackValue::Null => Err("expected integer on stack".to_string()),
    }
}

pub(crate) fn pop_shift_count(stack: &mut Vec<StackValue>) -> Result<i64, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(if value { 1 } else { 0 }),
        Some(StackValue::ByteString(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::BigInteger(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::Null) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Buffer(_, _)) => Err("expected integer-compatible value".to_string()),
        Some(_) => Err("expected integer-compatible shift count".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn pop_numeric_value(stack: &mut Vec<StackValue>) -> Result<i64, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(if value { 1 } else { 0 }),
        Some(StackValue::ByteString(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::BigInteger(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::Null) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Pointer(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Array(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Struct(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Map(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Buffer(_, _)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Interop(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Iterator(_)) => Err("expected integer-compatible value".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn pop_numeric_bigint(stack: &mut Vec<StackValue>) -> Result<BigInt, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(BigInt::from(value)),
        Some(StackValue::Boolean(value)) => Ok(BigInt::from(if value { 1 } else { 0 })),
        Some(StackValue::ByteString(value)) => decode_signed_le_bytes_bigint(&value),
        Some(StackValue::BigInteger(value)) => decode_signed_le_bytes_bigint(&value),
        Some(StackValue::Null) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Buffer(_, _)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Pointer(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Array(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Struct(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Map(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Interop(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Iterator(_)) => Err("expected integer-compatible value".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn shift_value_from_item(value: StackValue) -> Result<ShiftValue, String> {
    match value {
        StackValue::Integer(value) => Ok(ShiftValue(BigInt::from(value))),
        StackValue::Boolean(value) => Ok(ShiftValue(BigInt::from(if value { 1 } else { 0 }))),
        StackValue::ByteString(value) => Ok(ShiftValue(decode_signed_le_bytes_bigint(&value)?)),
        StackValue::BigInteger(value) => Ok(ShiftValue(decode_signed_le_bytes_bigint(&value)?)),
        StackValue::Null => Err("expected integer-compatible shift value".to_string()),
        StackValue::Pointer(_) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Array(..) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Struct(..) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Map(..) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Buffer(_, _) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Interop(_) => Err("expected integer-compatible shift value".to_string()),
        StackValue::Iterator(_) => Err("expected integer-compatible shift value".to_string()),
    }
}

pub(crate) fn num_equal(left: &StackValue, right: &StackValue) -> Result<bool, String> {
    Ok(integer_value_for_num_equal(left)? == integer_value_for_num_equal(right)?)
}

fn integer_value_for_num_equal(value: &StackValue) -> Result<BigInt, String> {
    match value {
        StackValue::Integer(value) => Ok(BigInt::from(*value)),
        StackValue::BigInteger(value) | StackValue::ByteString(value) => {
            decode_signed_le_bytes_bigint(value)
        }
        StackValue::Boolean(value) => Ok(BigInt::from(if *value { 1 } else { 0 })),
        StackValue::Null
        | StackValue::Pointer(_)
        | StackValue::Array(..)
        | StackValue::Struct(..)
        | StackValue::Map(..)
        | StackValue::Buffer(..)
        | StackValue::Interop(_)
        | StackValue::Iterator(_) => {
            Err("NUMEQUAL expects primitive numeric or byte string values".to_string())
        }
    }
}

#[inline]
pub(crate) fn integer_value_for_collection_index(value: &StackValue) -> Result<i64, String> {
    match value {
        StackValue::Integer(value) => Ok(*value),
        StackValue::Boolean(value) => Ok(if *value { 1 } else { 0 }),
        StackValue::ByteString(value) => decode_signed_le_bytes(value),
        StackValue::BigInteger(value) => decode_signed_le_bytes(value),
        StackValue::Null => Ok(0),
        _ => Err("expected integer-compatible collection index".to_string()),
    }
}

#[inline]
pub(crate) fn validate_map_key(key: &StackValue) -> Result<(), String> {
    match key {
        StackValue::Integer(_) | StackValue::Boolean(_) | StackValue::Null => Ok(()),
        StackValue::ByteString(value) => {
            if value.len() > 64 {
                Err("map key exceeds maximum size".to_string())
            } else {
                Ok(())
            }
        }
        _ => Err("map key must be primitive".to_string()),
    }
}

pub(crate) fn primitive_key_equals(left: &StackValue, right: &StackValue) -> bool {
    match (left, right) {
        (StackValue::Integer(left), StackValue::Integer(right)) => left == right,
        (StackValue::Boolean(left), StackValue::Boolean(right)) => left == right,
        (StackValue::Null, StackValue::Null) => true,
        (StackValue::ByteString(left), StackValue::ByteString(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn vm_equal(left: &StackValue, right: &StackValue) -> bool {
    match (left, right) {
        (StackValue::Integer(l), StackValue::Integer(r)) => l == r,
        (StackValue::Integer(l), StackValue::BigInteger(r))
        | (StackValue::BigInteger(r), StackValue::Integer(l)) => encode_integer(*l) == *r,
        (StackValue::BigInteger(l), StackValue::BigInteger(r)) => l == r,
        (StackValue::ByteString(l), StackValue::ByteString(r)) => l == r,
        (StackValue::Boolean(l), StackValue::Boolean(r)) => l == r,
        (StackValue::Pointer(l), StackValue::Pointer(r)) => l == r,
        (StackValue::Null, StackValue::Null) => true,
        (StackValue::Interop(l), StackValue::Interop(r)) => l == r,
        (StackValue::Iterator(l), StackValue::Iterator(r)) => l == r,
        (StackValue::Array(left_id, _), StackValue::Array(right_id, _))
        | (StackValue::Map(left_id, _), StackValue::Map(right_id, _))
        | (StackValue::Buffer(left_id, _), StackValue::Buffer(right_id, _)) => left_id == right_id,
        (StackValue::Struct(left_id, _), StackValue::Struct(right_id, _))
            if left_id == right_id =>
        {
            true
        }
        (StackValue::Struct(_, _), StackValue::Struct(_, _)) => struct_equal(left, right),
        _ => false,
    }
}

fn struct_equal(left: &StackValue, right: &StackValue) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (StackValue::Integer(l), StackValue::Integer(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Integer(l), StackValue::BigInteger(r))
            | (StackValue::BigInteger(r), StackValue::Integer(l)) => {
                if encode_integer(*l) != *r {
                    return false;
                }
            }
            (StackValue::BigInteger(l), StackValue::BigInteger(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::ByteString(l), StackValue::ByteString(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Boolean(l), StackValue::Boolean(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Pointer(l), StackValue::Pointer(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Null, StackValue::Null) => {}
            (StackValue::Interop(l), StackValue::Interop(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Iterator(l), StackValue::Iterator(r)) => {
                if l != r {
                    return false;
                }
            }
            (StackValue::Array(left_id, _), StackValue::Array(right_id, _))
            | (StackValue::Map(left_id, _), StackValue::Map(right_id, _))
            | (StackValue::Buffer(left_id, _), StackValue::Buffer(right_id, _)) => {
                if left_id != right_id {
                    return false;
                }
            }
            (StackValue::Struct(left_id, _), StackValue::Struct(right_id, _))
                if left_id == right_id => {}
            (StackValue::Struct(_, left_items), StackValue::Struct(_, right_items)) => {
                if left_items.len() != right_items.len() {
                    return false;
                }
                pending.extend(left_items.iter().zip(right_items.iter()));
            }
            _ => return false,
        }
    }
    true
}

pub(crate) fn convert_value(
    kind: u8,
    value: StackValue,
    ids: &mut CompoundIds,
) -> Result<StackValue, String> {
    // Validate target type first, even for Null
    match kind {
        0x20 | 0x21 | 0x28 | 0x30 | 0x40 | 0x41 | 0x48 | 0x60 => {}
        _ => return Err(format!("unsupported CONVERT target 0x{kind:02x}")),
    }

    if matches!(value, StackValue::Null) {
        return Ok(StackValue::Null);
    }

    match kind {
        0x20 => Ok(StackValue::Boolean(boolean_value(&value)?)),
        0x21 => Ok(match value {
            StackValue::Integer(value) => StackValue::Integer(value),
            StackValue::Boolean(value) => StackValue::Integer(if value { 1 } else { 0 }),
            StackValue::ByteString(bytes) | StackValue::BigInteger(bytes) => numeric_result_bigint(
                decode_signed_le_bytes_bigint(&bytes)?,
                "integer size exceeds maximum",
            )?,
            StackValue::Buffer(_, bytes) => numeric_result_bigint(
                decode_signed_le_bytes_bigint(&bytes)?,
                "integer size exceeds maximum",
            )?,
            other => return Err(format!("unsupported CONVERT source for Integer: {other:?}")),
        }),
        0x28 => Ok(match value {
            StackValue::ByteString(bytes) => StackValue::ByteString(bytes),
            StackValue::Buffer(_, bytes) => StackValue::ByteString(bytes),
            StackValue::Integer(value) => StackValue::ByteString(encode_integer(value)),
            StackValue::Boolean(value) => StackValue::ByteString(vec![if value { 1 } else { 0 }]),
            StackValue::BigInteger(value) => StackValue::ByteString(value),
            other => {
                return Err(format!(
                    "unsupported CONVERT source for ByteString: {other:?}"
                ))
            }
        }),
        0x30 => Ok(match value {
            StackValue::ByteString(bytes) => ids.buffer(bytes),
            StackValue::Buffer(_, _) => value,
            StackValue::Integer(value) => ids.buffer(encode_integer(value)),
            StackValue::BigInteger(value) => ids.buffer(value),
            StackValue::Boolean(value) => ids.buffer(vec![if value { 1 } else { 0 }]),
            other => return Err(format!("unsupported CONVERT source for Buffer: {other:?}")),
        }),
        0x40 => Ok(match value {
            StackValue::Array(_, _) => value,
            StackValue::Struct(_, items) => ids.array(items),
            other => return Err(format!("unsupported CONVERT source for Array: {other:?}")),
        }),
        0x41 => Ok(match value {
            StackValue::Struct(_, _) => value,
            StackValue::Array(_, items) => ids.r#struct(items),
            other => return Err(format!("unsupported CONVERT source for Struct: {other:?}")),
        }),
        0x48 => Ok(match value {
            StackValue::Map(_, _) => value,
            other => return Err(format!("unsupported CONVERT source for Map: {other:?}")),
        }),
        0x60 => Ok(match value {
            StackValue::Interop(_) => value,
            other => return Err(format!("unsupported CONVERT source for Interop: {other:?}")),
        }),
        _ => Err(format!("unsupported CONVERT target 0x{kind:02x}")),
    }
}

pub(crate) fn encode_retained_prefix_to_slice(
    stack: &[StackValue],
    buf: &mut [u8],
) -> Result<usize, String> {
    if buf.len() < 4 {
        return Err("retained prefix buffer too small".to_string());
    }

    let len = stack.len();
    if len > MAX_RETAINED_COLLECTION_LEN {
        return Err("collection length exceeds maximum".to_string());
    }

    buf[0..4].copy_from_slice(&(len as u32).to_le_bytes());
    let mut pos = 4;
    for value in stack {
        pos = encode_retained_value_to_slice(value, buf, pos)?;
    }
    Ok(pos)
}

pub(crate) fn encode_retained_value_to_slice(
    value: &StackValue,
    buf: &mut [u8],
    mut pos: usize,
) -> Result<usize, String> {
    match value {
        StackValue::Integer(value) => {
            ensure_retained_capacity(buf, pos, 9)?;
            buf[pos] = RETAINED_TAG_INTEGER;
            pos += 1;
            buf[pos..pos + 8].copy_from_slice(&value.to_le_bytes());
            Ok(pos + 8)
        }
        StackValue::BigInteger(bytes) => {
            pos = encode_retained_tag_and_len(RETAINED_TAG_BIGINTEGER, bytes.len(), buf, pos)?;
            ensure_retained_capacity(buf, pos, bytes.len())?;
            buf[pos..pos + bytes.len()].copy_from_slice(bytes);
            Ok(pos + bytes.len())
        }
        StackValue::ByteString(bytes) => {
            pos = encode_retained_tag_and_len(RETAINED_TAG_BYTESTRING, bytes.len(), buf, pos)?;
            ensure_retained_capacity(buf, pos, bytes.len())?;
            buf[pos..pos + bytes.len()].copy_from_slice(bytes);
            Ok(pos + bytes.len())
        }
        StackValue::Boolean(value) => {
            ensure_retained_capacity(buf, pos, 2)?;
            buf[pos] = RETAINED_TAG_BOOLEAN;
            buf[pos + 1] = u8::from(*value);
            Ok(pos + 2)
        }
        StackValue::Pointer(value) => {
            ensure_retained_capacity(buf, pos, 9)?;
            buf[pos] = RETAINED_TAG_POINTER;
            pos += 1;
            buf[pos..pos + 8].copy_from_slice(&(*value as u64).to_le_bytes());
            Ok(pos + 8)
        }
        StackValue::Array(id, items) => {
            pos = encode_retained_tag_id_len(RETAINED_TAG_ARRAY, *id, items.len(), buf, pos)?;
            for item in items {
                pos = encode_retained_value_to_slice(item, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Struct(id, items) => {
            pos = encode_retained_tag_id_len(RETAINED_TAG_STRUCT, *id, items.len(), buf, pos)?;
            for item in items {
                pos = encode_retained_value_to_slice(item, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Map(id, entries) => {
            pos = encode_retained_tag_id_len(RETAINED_TAG_MAP, *id, entries.len(), buf, pos)?;
            for (key, value) in entries {
                pos = encode_retained_value_to_slice(key, buf, pos)?;
                pos = encode_retained_value_to_slice(value, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Buffer(id, bytes) => {
            pos = encode_retained_tag_id_len(RETAINED_TAG_BUFFER, *id, bytes.len(), buf, pos)?;
            ensure_retained_capacity(buf, pos, bytes.len())?;
            buf[pos..pos + bytes.len()].copy_from_slice(bytes);
            Ok(pos + bytes.len())
        }
        StackValue::Interop(handle) => {
            ensure_retained_capacity(buf, pos, 9)?;
            buf[pos] = RETAINED_TAG_INTEROP;
            pos += 1;
            buf[pos..pos + 8].copy_from_slice(&handle.to_le_bytes());
            Ok(pos + 8)
        }
        StackValue::Iterator(handle) => {
            ensure_retained_capacity(buf, pos, 9)?;
            buf[pos] = RETAINED_TAG_ITERATOR;
            pos += 1;
            buf[pos..pos + 8].copy_from_slice(&handle.to_le_bytes());
            Ok(pos + 8)
        }
        StackValue::Null => {
            ensure_retained_capacity(buf, pos, 1)?;
            buf[pos] = RETAINED_TAG_NULL;
            Ok(pos + 1)
        }
    }
}

fn encode_retained_tag_and_len(
    tag: u8,
    len: usize,
    buf: &mut [u8],
    pos: usize,
) -> Result<usize, String> {
    if len > MAX_RETAINED_COLLECTION_LEN
        && matches!(
            tag,
            RETAINED_TAG_ARRAY | RETAINED_TAG_STRUCT | RETAINED_TAG_MAP
        )
    {
        return Err("collection length exceeds maximum".to_string());
    }
    let len = u32::try_from(len).map_err(|_| "retained prefix length exceeds u32".to_string())?;
    ensure_retained_capacity(buf, pos, 5)?;
    buf[pos] = tag;
    buf[pos + 1..pos + 5].copy_from_slice(&len.to_le_bytes());
    Ok(pos + 5)
}

fn encode_retained_tag_id_len(
    tag: u8,
    id: u64,
    len: usize,
    buf: &mut [u8],
    pos: usize,
) -> Result<usize, String> {
    if len > MAX_RETAINED_COLLECTION_LEN {
        return Err("collection length exceeds maximum".to_string());
    }
    let len = u32::try_from(len).map_err(|_| "retained prefix length exceeds u32".to_string())?;
    ensure_retained_capacity(buf, pos, 13)?;
    buf[pos] = tag;
    buf[pos + 1..pos + 9].copy_from_slice(&id.to_le_bytes());
    buf[pos + 9..pos + 13].copy_from_slice(&len.to_le_bytes());
    Ok(pos + 13)
}

pub(crate) fn ensure_retained_capacity(
    buf: &[u8],
    pos: usize,
    needed: usize,
) -> Result<(), String> {
    if buf.len().saturating_sub(pos) < needed {
        Err("retained prefix buffer too small".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
fn decode_retained_prefix(bytes: &[u8]) -> Result<Vec<StackValue>, String> {
    let mut stack = Vec::new();
    decode_retained_prefix_into(bytes, &mut stack)?;
    Ok(stack)
}

pub(crate) fn decode_retained_prefix_into(
    bytes: &[u8],
    stack: &mut Vec<StackValue>,
) -> Result<(), String> {
    if bytes.len() < 4 {
        return Err("truncated retained prefix length".to_string());
    }

    let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if len > MAX_RETAINED_COLLECTION_LEN {
        return Err("collection length exceeds maximum".to_string());
    }

    stack.clear();
    stack.reserve(len);

    let mut pos = 4;
    for _ in 0..len {
        stack.push(decode_retained_value(bytes, &mut pos, 0)?);
    }
    if pos != bytes.len() {
        return Err("retained prefix has trailing bytes".to_string());
    }
    Ok(())
}

fn decode_retained_value(
    bytes: &[u8],
    pos: &mut usize,
    depth: usize,
) -> Result<StackValue, String> {
    if depth > MAX_RETAINED_DECODE_DEPTH {
        return Err("decode depth exceeds maximum".to_string());
    }
    if *pos >= bytes.len() {
        return Err("truncated retained prefix tag".to_string());
    }

    let tag = bytes[*pos];
    *pos += 1;
    match tag {
        RETAINED_TAG_INTEGER => {
            let value = decode_retained_i64(bytes, pos)?;
            Ok(StackValue::Integer(value))
        }
        RETAINED_TAG_BIGINTEGER => {
            let data = decode_retained_bytes(bytes, pos)?;
            Ok(StackValue::BigInteger(data))
        }
        RETAINED_TAG_BYTESTRING => {
            let data = decode_retained_bytes(bytes, pos)?;
            Ok(StackValue::ByteString(data))
        }
        RETAINED_TAG_BOOLEAN => {
            let value = decode_retained_u8(bytes, pos)?;
            Ok(StackValue::Boolean(value != 0))
        }
        RETAINED_TAG_POINTER => {
            let value = decode_retained_u64(bytes, pos)?;
            let pointer = usize::try_from(value).map_err(|_| "pointer out of range".to_string())?;
            Ok(StackValue::Pointer(pointer))
        }
        RETAINED_TAG_ARRAY => {
            let id = decode_retained_u64(bytes, pos)?;
            let len = decode_retained_u32(bytes, pos)? as usize;
            if len > MAX_RETAINED_COLLECTION_LEN {
                return Err("collection length exceeds maximum".to_string());
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_retained_value(bytes, pos, depth + 1)?);
            }
            Ok(StackValue::Array(id, items))
        }
        RETAINED_TAG_STRUCT => {
            let id = decode_retained_u64(bytes, pos)?;
            let len = decode_retained_u32(bytes, pos)? as usize;
            if len > MAX_RETAINED_COLLECTION_LEN {
                return Err("collection length exceeds maximum".to_string());
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_retained_value(bytes, pos, depth + 1)?);
            }
            Ok(StackValue::Struct(id, items))
        }
        RETAINED_TAG_MAP => {
            let id = decode_retained_u64(bytes, pos)?;
            let len = decode_retained_u32(bytes, pos)? as usize;
            if len > MAX_RETAINED_COLLECTION_LEN {
                return Err("collection length exceeds maximum".to_string());
            }
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                let key = decode_retained_value(bytes, pos, depth + 1)?;
                let value = decode_retained_value(bytes, pos, depth + 1)?;
                entries.push((key, value));
            }
            Ok(StackValue::Map(id, entries))
        }
        RETAINED_TAG_BUFFER => {
            let id = decode_retained_u64(bytes, pos)?;
            let data = decode_retained_bytes(bytes, pos)?;
            Ok(StackValue::Buffer(id, data))
        }
        RETAINED_TAG_INTEROP => {
            let handle = decode_retained_u64(bytes, pos)?;
            Ok(StackValue::Interop(handle))
        }
        RETAINED_TAG_ITERATOR => {
            let handle = decode_retained_u64(bytes, pos)?;
            Ok(StackValue::Iterator(handle))
        }
        RETAINED_TAG_NULL => Ok(StackValue::Null),
        _ => Err(format!("invalid retained prefix tag 0x{tag:02x}")),
    }
}

fn decode_retained_bytes(bytes: &[u8], pos: &mut usize) -> Result<Vec<u8>, String> {
    let len = decode_retained_u32(bytes, pos)? as usize;
    if bytes.len().saturating_sub(*pos) < len {
        return Err("truncated retained prefix bytes".to_string());
    }
    let data = bytes[*pos..*pos + len].to_vec();
    *pos += len;
    Ok(data)
}

fn decode_retained_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, String> {
    ensure_retained_input(bytes, *pos, 1)?;
    let value = bytes[*pos];
    *pos += 1;
    Ok(value)
}

fn decode_retained_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    ensure_retained_input(bytes, *pos, 4)?;
    let value = u32::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
    ]);
    *pos += 4;
    Ok(value)
}

fn decode_retained_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, String> {
    ensure_retained_input(bytes, *pos, 8)?;
    let value = u64::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
        bytes[*pos + 4],
        bytes[*pos + 5],
        bytes[*pos + 6],
        bytes[*pos + 7],
    ]);
    *pos += 8;
    Ok(value)
}

fn decode_retained_i64(bytes: &[u8], pos: &mut usize) -> Result<i64, String> {
    ensure_retained_input(bytes, *pos, 8)?;
    let value = i64::from_le_bytes([
        bytes[*pos],
        bytes[*pos + 1],
        bytes[*pos + 2],
        bytes[*pos + 3],
        bytes[*pos + 4],
        bytes[*pos + 5],
        bytes[*pos + 6],
        bytes[*pos + 7],
    ]);
    *pos += 8;
    Ok(value)
}

fn ensure_retained_input(bytes: &[u8], pos: usize, needed: usize) -> Result<(), String> {
    if bytes.len().saturating_sub(pos) < needed {
        Err("truncated retained prefix".to_string())
    } else {
        Ok(())
    }
}

#[inline]
fn stabilize_allocator_after_host_call() {
    if cfg!(target_arch = "riscv32") {
        let _scratch = Vec::<u8>::with_capacity(4096);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn invoke_syscall<H: SyscallProvider>(
    host: &mut H,
    api: u32,
    ip: usize,
    stack: &mut Vec<StackValue>,
    args: &mut Vec<StackValue>,
    locals: &mut Vec<StackValue>,
    static_fields: &mut Vec<StackValue>,
    alt_stack: &mut Vec<StackValue>,
    consumed_mutations: &mut Vec<StackValue>,
    ids: &mut CompoundIds,
) -> Result<(), String> {
    let arg_count = neo_riscv_abi::syscall_arg_count(api).min(stack.len());
    let keep = stack.len() - arg_count;

    let mut abi_args: Vec<neo_riscv_abi::StackValue> = Vec::with_capacity(arg_count.min(64));
    for item in &stack[keep..] {
        abi_args.push(to_abi_value(item));
    }

    let retained_stack_len = if cfg!(target_arch = "riscv32") {
        let buf = unsafe { RETAINED_STACK_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(stack, buf)?)
    } else {
        None
    };
    let retained_locals_len = if cfg!(target_arch = "riscv32") && !locals.is_empty() {
        let buf = unsafe { RETAINED_LOCALS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(locals, buf)?)
    } else {
        None
    };
    let retained_args_len = if cfg!(target_arch = "riscv32") && !args.is_empty() {
        let buf = unsafe { RETAINED_ARGS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(args, buf)?)
    } else {
        None
    };
    let retained_static_fields_len = if cfg!(target_arch = "riscv32") && !static_fields.is_empty() {
        let buf = unsafe { RETAINED_STATIC_FIELDS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(static_fields, buf)?)
    } else {
        None
    };
    let retained_alt_stack_len = if cfg!(target_arch = "riscv32") && !alt_stack.is_empty() {
        let buf = unsafe { RETAINED_ALT_STACK_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(alt_stack, buf)?)
    } else {
        None
    };
    let retained_consumed_mutations_len =
        if cfg!(target_arch = "riscv32") && !consumed_mutations.is_empty() {
            let buf = unsafe { RETAINED_CONSUMED_MUTATIONS_BUF.as_mut_slice() };
            Some(encode_retained_prefix_to_slice(consumed_mutations, buf)?)
        } else {
            None
        };
    match host.syscall(api, ip, &mut abi_args) {
        Ok(()) => {
            stabilize_allocator_after_host_call();
            if let Some(retained_len) = retained_stack_len {
                restore_retained_values(
                    consumed_mutations,
                    retained_consumed_mutations_len,
                    &RETAINED_CONSUMED_MUTATIONS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    locals,
                    retained_locals_len,
                    &RETAINED_LOCALS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    args,
                    retained_args_len,
                    &RETAINED_ARGS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    static_fields,
                    retained_static_fields_len,
                    &RETAINED_STATIC_FIELDS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    alt_stack,
                    retained_alt_stack_len,
                    &RETAINED_ALT_STACK_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    stack,
                    Some(retained_len),
                    &RETAINED_STACK_BUF,
                    keep.saturating_add(abi_args.len())
                        .max(POST_SYSCALL_STACK_HEADROOM),
                )?;
                stack.truncate(keep);
                stack.reserve(abi_args.len());
                for item in abi_args {
                    stack.push(ids.import_abi(item));
                }
                return Ok(());
            }

            stack.truncate(keep);
            stack.reserve(abi_args.len());
            for item in abi_args {
                stack.push(ids.import_abi(item));
            }
            Ok(())
        }
        Err(e) => {
            if let Some(retained_len) = retained_stack_len {
                let _ = restore_retained_values(
                    consumed_mutations,
                    retained_consumed_mutations_len,
                    &RETAINED_CONSUMED_MUTATIONS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    locals,
                    retained_locals_len,
                    &RETAINED_LOCALS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    args,
                    retained_args_len,
                    &RETAINED_ARGS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    static_fields,
                    retained_static_fields_len,
                    &RETAINED_STATIC_FIELDS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    alt_stack,
                    retained_alt_stack_len,
                    &RETAINED_ALT_STACK_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    stack,
                    Some(retained_len),
                    &RETAINED_STACK_BUF,
                    stack.len().max(POST_SYSCALL_STACK_HEADROOM),
                );
            }
            core::mem::forget(abi_args);
            Err(e)
        }
    }
}

pub(crate) fn complete_initializer_retaining_state<H: SyscallProvider>(
    host: &mut H,
    ip: usize,
    method_initial_stack: &mut Vec<StackValue>,
    static_fields: &mut Vec<StackValue>,
) -> Result<(), String> {
    let retained_method_stack_len =
        if cfg!(target_arch = "riscv32") && !method_initial_stack.is_empty() {
            let buf = unsafe { RETAINED_STACK_BUF.as_mut_slice() };
            Some(encode_retained_prefix_to_slice(method_initial_stack, buf)?)
        } else {
            None
        };
    let retained_static_fields_len = if cfg!(target_arch = "riscv32") && !static_fields.is_empty() {
        let buf = unsafe { RETAINED_STATIC_FIELDS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(static_fields, buf)?)
    } else {
        None
    };

    let result = host.initializer_complete(ip);
    if result.is_ok() {
        stabilize_allocator_after_host_call();
    }

    if retained_method_stack_len.is_some() || retained_static_fields_len.is_some() {
        let method_restore = restore_retained_values(
            method_initial_stack,
            retained_method_stack_len,
            &RETAINED_STACK_BUF,
            method_initial_stack.len().max(POST_SYSCALL_STACK_HEADROOM),
        );
        let static_restore = restore_retained_values(
            static_fields,
            retained_static_fields_len,
            &RETAINED_STATIC_FIELDS_BUF,
            static_fields.len().max(POST_SYSCALL_STACK_HEADROOM),
        );

        if let Err(error) = result {
            let _ = method_restore;
            let _ = static_restore;
            return Err(error);
        }

        method_restore?;
        static_restore?;
        return Ok(());
    }

    result
}

pub(crate) fn retain_initializer_method_stack(
    method_initial_stack: &[StackValue],
) -> Result<Option<usize>, String> {
    #[cfg(target_arch = "riscv32")]
    {
        if method_initial_stack.is_empty() {
            return Ok(None);
        }
        let buf = unsafe { RETAINED_INITIAL_STACK_BUF.as_mut_slice() };
        return encode_retained_prefix_to_slice(method_initial_stack, buf).map(Some);
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        let _ = method_initial_stack;
        Ok(None)
    }
}

pub(crate) fn restore_initializer_method_stack(
    method_initial_stack: &mut Vec<StackValue>,
    retained_len: Option<usize>,
) -> Result<(), String> {
    #[cfg(target_arch = "riscv32")]
    {
        restore_retained_values(
            method_initial_stack,
            retained_len,
            &RETAINED_INITIAL_STACK_BUF,
            method_initial_stack.len().max(POST_SYSCALL_STACK_HEADROOM),
        )
    }
    #[cfg(not(target_arch = "riscv32"))]
    {
        let _ = (method_initial_stack, retained_len);
        Ok(())
    }
}

fn restore_retained_values(
    values: &mut Vec<StackValue>,
    retained_len: Option<usize>,
    buf: &RetainedPrefixBuffer,
    min_capacity: usize,
) -> Result<(), String> {
    match retained_len {
        Some(retained_len) => {
            let retired = core::mem::take(values);
            core::mem::forget(retired);

            let mut restored = Vec::with_capacity(min_capacity);
            decode_retained_prefix_into(unsafe { buf.as_slice(retained_len) }, &mut restored)?;
            *values = restored;
            Ok(())
        }
        None => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn invoke_callt<H: SyscallProvider>(
    host: &mut H,
    token: u16,
    ip: usize,
    stack: &mut Vec<StackValue>,
    args: &mut Vec<StackValue>,
    locals: &mut Vec<StackValue>,
    static_fields: &mut Vec<StackValue>,
    alt_stack: &mut Vec<StackValue>,
    consumed_mutations: &mut Vec<StackValue>,
    ids: &mut CompoundIds,
) -> Result<(), String> {
    let mut abi_stack = to_abi_stack(stack);
    let retained_stack_len = if cfg!(target_arch = "riscv32") {
        let buf = unsafe { RETAINED_STACK_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(stack, buf)?)
    } else {
        None
    };
    let retained_locals_len = if cfg!(target_arch = "riscv32") && !locals.is_empty() {
        let buf = unsafe { RETAINED_LOCALS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(locals, buf)?)
    } else {
        None
    };
    let retained_args_len = if cfg!(target_arch = "riscv32") && !args.is_empty() {
        let buf = unsafe { RETAINED_ARGS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(args, buf)?)
    } else {
        None
    };
    let retained_static_fields_len = if cfg!(target_arch = "riscv32") && !static_fields.is_empty() {
        let buf = unsafe { RETAINED_STATIC_FIELDS_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(static_fields, buf)?)
    } else {
        None
    };
    let retained_alt_stack_len = if cfg!(target_arch = "riscv32") && !alt_stack.is_empty() {
        let buf = unsafe { RETAINED_ALT_STACK_BUF.as_mut_slice() };
        Some(encode_retained_prefix_to_slice(alt_stack, buf)?)
    } else {
        None
    };
    let retained_consumed_mutations_len =
        if cfg!(target_arch = "riscv32") && !consumed_mutations.is_empty() {
            let buf = unsafe { RETAINED_CONSUMED_MUTATIONS_BUF.as_mut_slice() };
            Some(encode_retained_prefix_to_slice(consumed_mutations, buf)?)
        } else {
            None
        };
    match host.callt(token, ip, &mut abi_stack) {
        Ok(()) => {
            stabilize_allocator_after_host_call();
            if let Some(retained_len) = retained_stack_len {
                restore_retained_values(
                    consumed_mutations,
                    retained_consumed_mutations_len,
                    &RETAINED_CONSUMED_MUTATIONS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    locals,
                    retained_locals_len,
                    &RETAINED_LOCALS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    args,
                    retained_args_len,
                    &RETAINED_ARGS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    static_fields,
                    retained_static_fields_len,
                    &RETAINED_STATIC_FIELDS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    alt_stack,
                    retained_alt_stack_len,
                    &RETAINED_ALT_STACK_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                )?;
                restore_retained_values(
                    stack,
                    Some(retained_len),
                    &RETAINED_STACK_BUF,
                    stack.len().max(POST_SYSCALL_STACK_HEADROOM),
                )?;
            }

            let mut imported_stack =
                Vec::with_capacity(abi_stack.len().max(POST_SYSCALL_STACK_HEADROOM));
            for item in abi_stack {
                let imported = ids.import_abi(item);
                let stabilized = if cfg!(target_arch = "riscv32")
                    && matches!(
                        imported,
                        StackValue::Array(..)
                            | StackValue::Struct(..)
                            | StackValue::Map(..)
                            | StackValue::Buffer(..)
                    ) {
                    ids.deep_clone(&imported)
                } else {
                    imported
                };
                imported_stack.push(stabilized);
            }
            let shared_prefix = stack
                .iter()
                .zip(imported_stack.iter())
                .take_while(|(left, right)| structurally_equal(left, right))
                .count();
            #[cfg(target_arch = "riscv32")]
            {
                let mut preserved_stack = core::mem::take(stack);
                let mut next_stack =
                    Vec::with_capacity(imported_stack.len().max(POST_SYSCALL_STACK_HEADROOM));

                if shared_prefix > 0 {
                    let discarded_preserved = preserved_stack.split_off(shared_prefix);
                    next_stack.append(&mut preserved_stack);
                    core::mem::forget(discarded_preserved);
                } else {
                    core::mem::forget(preserved_stack);
                }

                let mut imported_suffix = imported_stack.split_off(shared_prefix);
                next_stack.append(&mut imported_suffix);
                core::mem::forget(imported_stack);
                *stack = next_stack;
            }
            #[cfg(not(target_arch = "riscv32"))]
            {
                imported_stack[..shared_prefix].clone_from_slice(&stack[..shared_prefix]);
                *stack = imported_stack;
            }
            Ok(())
        }
        Err(e) => {
            if let Some(retained_len) = retained_stack_len {
                let _ = restore_retained_values(
                    consumed_mutations,
                    retained_consumed_mutations_len,
                    &RETAINED_CONSUMED_MUTATIONS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    locals,
                    retained_locals_len,
                    &RETAINED_LOCALS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    args,
                    retained_args_len,
                    &RETAINED_ARGS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    static_fields,
                    retained_static_fields_len,
                    &RETAINED_STATIC_FIELDS_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    alt_stack,
                    retained_alt_stack_len,
                    &RETAINED_ALT_STACK_BUF,
                    POST_SYSCALL_STACK_HEADROOM,
                );
                let _ = restore_retained_values(
                    stack,
                    Some(retained_len),
                    &RETAINED_STACK_BUF,
                    stack.len().max(POST_SYSCALL_STACK_HEADROOM),
                );
            }
            // Leak abi_stack on error to avoid talc free-list corruption on RISC-V/PolkaVM
            core::mem::forget(abi_stack);
            Err(e)
        }
    }
}

#[inline]
pub(crate) fn boolean_value(value: &StackValue) -> Result<bool, String> {
    match value {
        StackValue::Boolean(value) => Ok(*value),
        StackValue::Integer(value) => Ok(*value != 0),
        StackValue::BigInteger(value) => Ok(value.iter().any(|byte| *byte != 0)),
        StackValue::ByteString(value) => Ok(value.iter().any(|byte| *byte != 0)),
        StackValue::Buffer(_, _) => Ok(true), // Buffer is a compound type, always true
        StackValue::Pointer(_) => Ok(true),
        StackValue::Array(..) => Ok(true),
        StackValue::Struct(..) => Ok(true),
        StackValue::Map(..) => Ok(true),
        StackValue::Interop(_) => Ok(true),
        StackValue::Iterator(_) => Ok(true),
        StackValue::Null => Ok(false),
    }
}

pub(crate) fn decode_signed_le_bytes(bytes: &[u8]) -> Result<i64, String> {
    if bytes.is_empty() {
        return Ok(0);
    }
    if bytes.len() > MAX_INTEGER_SIZE {
        return Err("integer size exceeds maximum".to_string());
    }

    let sign_extend = if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };

    if bytes.len() > 8 {
        if bytes.iter().all(|byte| *byte == 0) {
            return Ok(0);
        }

        if bytes[8..].iter().all(|byte| *byte == sign_extend)
            && ((bytes[7] & 0x80) == (sign_extend & 0x80))
        {
            let mut buffer = [sign_extend; 8];
            buffer.copy_from_slice(&bytes[..8]);
            return Ok(i64::from_le_bytes(buffer));
        }

        return Err("integer exceeds i64 range".to_string());
    }

    let mut buffer = [sign_extend; 8];
    buffer[..bytes.len()].copy_from_slice(bytes);
    Ok(i64::from_le_bytes(buffer))
}

pub(crate) fn decode_signed_le_bytes_bigint(bytes: &[u8]) -> Result<BigInt, String> {
    if bytes.len() > MAX_INTEGER_SIZE {
        return Err("integer size exceeds maximum".to_string());
    }
    Ok(BigInt::from_signed_bytes_le(bytes))
}

pub(crate) struct ShiftValue(BigInt);

impl ShiftValue {
    pub(crate) fn shift_left(self, shift: u32) -> Result<StackValue, String> {
        numeric_result_bigint(self.0 << (shift as usize), "integer overflow for SHL")
    }

    pub(crate) fn shift_right(self, shift: u32) -> Result<StackValue, String> {
        numeric_result_bigint(self.0 >> (shift as usize), "integer overflow for SHR")
    }
}

#[inline]
pub(crate) fn pop_boolean(stack: &mut Vec<StackValue>) -> Result<bool, String> {
    match stack.pop() {
        Some(StackValue::Boolean(value)) => Ok(value),
        Some(StackValue::Integer(value)) => Ok(value != 0),
        Some(StackValue::BigInteger(value)) => Ok(value.iter().any(|byte| *byte != 0)),
        Some(StackValue::ByteString(value)) => Ok(value.iter().any(|byte| *byte != 0)),
        Some(StackValue::Buffer(_, value)) => Ok(value.iter().any(|byte| *byte != 0)),
        Some(StackValue::Null) => Ok(false),
        Some(_) => Ok(true),
        None => Err("stack underflow".to_string()),
    }
}

/// Convert a StackValue to boolean via the integer path (NeoVM GetBoolean).
/// ByteString/BigInteger > 32 bytes will FAULT, matching NeoVM's MaxSize check.
pub(crate) fn item_to_boolean_strict(item: &StackValue) -> Result<bool, String> {
    match item {
        StackValue::Boolean(value) => Ok(*value),
        StackValue::Integer(value) => Ok(*value != 0),
        StackValue::BigInteger(value) => {
            if value.len() > MAX_INTEGER_SIZE {
                return Err("integer size exceeds maximum".to_string());
            }
            Ok(value.iter().any(|byte| *byte != 0))
        }
        StackValue::ByteString(value) => {
            if value.len() > MAX_INTEGER_SIZE {
                return Err("integer size exceeds maximum".to_string());
            }
            Ok(value.iter().any(|byte| *byte != 0))
        }
        StackValue::Buffer(_, value) => {
            if value.len() > MAX_INTEGER_SIZE {
                return Err("integer size exceeds maximum".to_string());
            }
            Ok(value.iter().any(|byte| *byte != 0))
        }
        StackValue::Null => Ok(false),
        _ => Ok(true),
    }
}

pub(crate) fn mod_pow(base: i64, exponent: i64, modulus: i64) -> Result<i64, String> {
    if modulus == 0 {
        return Err("division by zero for MODPOW".to_string());
    }

    if exponent == -1 {
        return mod_inverse(base, modulus);
    }

    if exponent < 0 {
        return Err("negative exponent for MODPOW".to_string());
    }

    let modulus = i128::from(modulus);
    let mut result: i128 = 1 % modulus;
    let mut power = i128::from(base);
    let mut exponent = exponent as u64;

    while exponent > 0 {
        if exponent & 1 == 1 {
            result = (result * power) % modulus;
        }
        exponent >>= 1;
        if exponent > 0 {
            power = (power * power) % modulus;
        }
    }

    i64::try_from(result).map_err(|_| "integer overflow for MODPOW".to_string())
}

pub(crate) fn mod_inverse(value: i64, modulus: i64) -> Result<i64, String> {
    if value <= 0 {
        return Err("value has no modular inverse".to_string());
    }
    if modulus <= 1 {
        return Err("invalid modulus for modular inverse".to_string());
    }

    let mut t: i128 = 0;
    let mut new_t: i128 = 1;
    let mut r: i128 = i128::from(modulus);
    let mut new_r: i128 = i128::from(value);

    while new_r != 0 {
        let quotient = r / new_r;
        (t, new_t) = (new_t, t - quotient * new_t);
        (r, new_r) = (new_r, r - quotient * new_r);
    }

    if r != 1 && r != -1 {
        return Err("value is not invertible for MODPOW".to_string());
    }

    let modulus = i128::from(modulus);
    let mut inverse = t % modulus;
    if inverse == 0 {
        return Ok(0);
    }
    if (modulus > 0 && inverse < 0) || (modulus < 0 && inverse > 0) {
        inverse += modulus;
    }

    i64::try_from(inverse).map_err(|_| "integer overflow for MODPOW".to_string())
}

pub(crate) fn pop_bytes(stack: &mut Vec<StackValue>) -> Result<Vec<u8>, String> {
    match stack.pop() {
        Some(StackValue::ByteString(value)) => Ok(value),
        Some(StackValue::Buffer(_, value)) => Ok(value),
        Some(StackValue::Integer(value)) => Ok(encode_integer(value)),
        Some(StackValue::BigInteger(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(vec![if value { 1 } else { 0 }]),
        Some(StackValue::Null) => Ok(Vec::new()),
        Some(
            StackValue::Pointer(_)
            | StackValue::Array(..)
            | StackValue::Struct(..)
            | StackValue::Map(..)
            | StackValue::Interop(_)
            | StackValue::Iterator(_),
        ) => Err("expected byte string-compatible item on stack".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

/// Convert a StackValue to bytes without consuming it from the stack.
/// Used by CAT to determine result type while extracting byte content.
pub(crate) fn stack_item_to_bytes(item: StackValue) -> Result<Vec<u8>, String> {
    match item {
        StackValue::ByteString(value) => Ok(value),
        StackValue::Buffer(_, value) => Ok(value),
        StackValue::Integer(value) => Ok(encode_integer(value)),
        StackValue::BigInteger(value) => Ok(value),
        StackValue::Boolean(value) => Ok(vec![if value { 1 } else { 0 }]),
        StackValue::Null => Ok(Vec::new()),
        StackValue::Pointer(_)
        | StackValue::Array(..)
        | StackValue::Struct(..)
        | StackValue::Map(..)
        | StackValue::Interop(_)
        | StackValue::Iterator(_) => {
            Err("expected byte string-compatible item on stack".to_string())
        }
    }
}

pub(crate) fn encode_integer(value: i64) -> Vec<u8> {
    if value == 0 {
        return Vec::new();
    }

    let mut bytes = value.to_le_bytes().to_vec();
    if value > 0 {
        while bytes.len() > 1 && bytes.last() == Some(&0) {
            if bytes[bytes.len() - 2] & 0x80 != 0 {
                break;
            }
            bytes.pop();
        }
    } else {
        while bytes.len() > 1 && bytes.last() == Some(&0xff) {
            if bytes[bytes.len() - 2] & 0x80 == 0 {
                break;
            }
            bytes.pop();
        }
    }

    bytes
}

pub(crate) fn numeric_result_bigint(
    value: BigInt,
    overflow_message: &str,
) -> Result<StackValue, String> {
    let bytes = trim_le_bytes(value.to_signed_bytes_le());
    if bytes.len() > MAX_INTEGER_SIZE {
        return Err(overflow_message.to_string());
    }
    Ok(bigint_or_integer(bytes))
}

pub(crate) fn bigint_sign(value: &BigInt) -> i64 {
    if value.is_zero() {
        0
    } else if value < &BigInt::zero() {
        -1
    } else {
        1
    }
}

pub(crate) fn bigint_abs(value: BigInt) -> BigInt {
    if value < BigInt::zero() {
        -value
    } else {
        value
    }
}

/// Distinguishes short (i8, 1-byte) from long (i32, 4-byte) jump offsets.
pub(crate) enum Offset {
    Short,
    Long,
}

/// Read a jump/call offset from the script at `ip`.
/// Returns (signed offset as isize, byte advance for operand).
pub(crate) fn read_offset(
    script: &[u8],
    ip: usize,
    kind: &Offset,
    name: &str,
) -> Result<(isize, usize), String> {
    match kind {
        Offset::Short => {
            if ip + 2 > script.len() {
                return Err(format!("truncated {name} operand"));
            }
            let offset = i8::from_le_bytes([script[ip + 1]]);
            Ok((offset as isize, 2))
        }
        Offset::Long => {
            if ip + 5 > script.len() {
                return Err(format!("truncated {name}_L operand"));
            }
            let offset = i32::from_le_bytes([
                script[ip + 1],
                script[ip + 2],
                script[ip + 3],
                script[ip + 4],
            ]);
            Ok((offset as isize, 5))
        }
    }
}

/// Compute jump/call target from ip + offset with bounds checking.
pub(crate) fn compute_jump_target_offset(
    ip: usize,
    offset: isize,
    script_len: usize,
    name: &str,
) -> Result<usize, String> {
    let target = ip as isize + offset;
    if target < 0 || target as usize > script_len {
        return Err(format!("{name} target out of bounds"));
    }
    Ok(target as usize)
}

pub(crate) fn trim_le_bytes(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.is_empty() {
        return bytes;
    }

    let sign_extend = if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };
    while bytes.len() > 1 && bytes.last() == Some(&sign_extend) {
        let next = bytes[bytes.len() - 2];
        if (next & 0x80 != 0) == (sign_extend == 0xff) {
            bytes.pop();
        } else {
            break;
        }
    }
    bytes
}

/// Convert LE two's complement bytes to i64.
/// Values that don't fit in i64 are truncated to the lower 8 bytes.
pub(crate) fn bytes_to_integer(bytes: &[u8]) -> i64 {
    if bytes.is_empty() {
        return 0;
    }

    // Determine sign from the last byte's high bit
    let negative = bytes.last().is_some_and(|b| b & 0x80 != 0);

    let mut buf = [0u8; 8];
    let copy_len = bytes.len().min(8);
    buf[..copy_len].copy_from_slice(&bytes[..copy_len]);

    // Sign-extend if negative
    if negative && copy_len < 8 {
        for b in &mut buf[copy_len..] {
            *b = 0xFF;
        }
    }

    i64::from_le_bytes(buf)
}

pub(crate) fn bitwise_result<F>(
    left: &StackValue,
    right: &StackValue,
    op: F,
) -> Result<StackValue, String>
where
    F: Fn(u8, u8) -> u8,
{
    let left_bytes = bitwise_operand_bytes(left)?;
    let right_bytes = bitwise_operand_bytes(right)?;
    Ok(bigint_or_integer(bitwise_signed_bytes(
        &left_bytes,
        &right_bytes,
        op,
    )?))
}

fn bitwise_operand_bytes(value: &StackValue) -> Result<Vec<u8>, String> {
    match value {
        StackValue::Integer(value) => Ok(encode_integer(*value)),
        StackValue::BigInteger(value) | StackValue::ByteString(value) => Ok(value.clone()),
        StackValue::Boolean(value) => Ok(encode_integer(if *value { 1 } else { 0 })),
        _ => Err("bitwise op expects primitive numeric or byte string operands".to_string()),
    }
}

pub(crate) fn bigint_or_integer(bytes: Vec<u8>) -> StackValue {
    let trimmed = trim_le_bytes(bytes);
    if trimmed.len() <= 8 {
        StackValue::Integer(bytes_to_integer(&trimmed))
    } else {
        StackValue::BigInteger(trimmed)
    }
}

fn bitwise_signed_bytes<F>(left: &[u8], right: &[u8], op: F) -> Result<Vec<u8>, String>
where
    F: Fn(u8, u8) -> u8,
{
    let len = left.len().max(right.len());
    if len == 0 {
        return Ok(vec![op(0, 0)]);
    }
    if len > 32 {
        return Err("bitwise operand exceeds supported size".to_string());
    }

    let left_fill = if left.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };
    let right_fill = if right.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let lb = left.get(i).copied().unwrap_or(left_fill);
        let rb = right.get(i).copied().unwrap_or(right_fill);
        result.push(op(lb, rb));
    }
    Ok(result)
}

#[inline]
pub(crate) fn trim_le_bytes_slice(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }

    let sign_extend = if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };
    let mut end = bytes.len();
    while end > 1 && bytes[end - 1] == sign_extend {
        let next = bytes[end - 2];
        if (next & 0x80 != 0) == (sign_extend == 0xff) {
            end -= 1;
        } else {
            break;
        }
    }
    bytes[..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::{
        decode_retained_prefix, encode_retained_prefix_to_slice, StackValue,
        MAX_RETAINED_COLLECTION_LEN,
    };
    use alloc::vec;

    #[test]
    fn retained_prefix_codec_round_trips_nested_compound_ids() {
        let shared_buffer = StackValue::Buffer(7, b"value".to_vec());
        let stack = vec![
            shared_buffer.clone(),
            StackValue::Array(
                11,
                vec![
                    shared_buffer.clone(),
                    StackValue::Struct(
                        12,
                        vec![
                            StackValue::Map(
                                13,
                                vec![(shared_buffer.clone(), StackValue::Integer(5))],
                            ),
                            StackValue::Null,
                        ],
                    ),
                ],
            ),
        ];

        let mut buf = vec![0u8; 1024];
        let len = encode_retained_prefix_to_slice(&stack, &mut buf)
            .expect("retained prefix should encode into the scratch buffer");
        let decoded = decode_retained_prefix(&buf[..len])
            .expect("retained prefix should decode from the scratch buffer");

        assert_eq!(decoded, stack);
    }

    #[test]
    fn retained_prefix_codec_rejects_oversized_top_level_stack() {
        let stack = vec![StackValue::Null; MAX_RETAINED_COLLECTION_LEN + 1];
        let mut buf = vec![0u8; 1024];

        let err = encode_retained_prefix_to_slice(&stack, &mut buf)
            .expect_err("oversized retained prefixes must be rejected");

        assert_eq!(err, "collection length exceeds maximum");
    }

    #[test]
    fn retained_prefix_codec_round_trips_block_like_struct_argument_list() {
        let prev_hash = vec![
            0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c,
            0x7d, 0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4,
            0xef, 0x1d, 0x4d, 0x1f,
        ];
        let block_like = StackValue::Struct(
            42,
            vec![
                StackValue::ByteString(vec![0xd9; 32]),
                StackValue::Integer(0),
                StackValue::ByteString(prev_hash.clone()),
                StackValue::ByteString(vec![0x72; 32]),
                StackValue::Integer(1),
                StackValue::Integer(2),
                StackValue::Integer(3),
                StackValue::Integer(4),
                StackValue::ByteString(vec![0x6b; 20]),
                StackValue::Integer(1),
            ],
        );
        let stack = vec![block_like, StackValue::ByteString(b"PrevHash".to_vec())];

        let mut buf = vec![0u8; 4096];
        let len = encode_retained_prefix_to_slice(&stack, &mut buf)
            .expect("block-like retained prefix should encode");
        let decoded =
            decode_retained_prefix(&buf[..len]).expect("block-like retained prefix should decode");

        assert_eq!(decoded, stack);
    }
}
