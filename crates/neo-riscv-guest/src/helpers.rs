extern crate alloc;

use crate::runtime_types::{to_abi_stack, CompoundIds, StackValue};
use crate::SyscallProvider;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

pub(crate) fn peek_item(stack: &[StackValue]) -> Result<StackValue, String> {
    stack
        .last()
        .cloned()
        .ok_or_else(|| "stack underflow".to_string())
}

pub(crate) fn pop_item(stack: &mut Vec<StackValue>) -> Result<StackValue, String> {
    stack.pop().ok_or_else(|| "stack underflow".to_string())
}

pub(crate) fn pop_integer(stack: &mut Vec<StackValue>) -> Result<i64, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(if value { 1 } else { 0 }),
        Some(_) => Err("expected integer on stack".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn pop_integer_pair_allowing_null_false(
    stack: &mut Vec<StackValue>,
) -> Result<Option<(i64, i64)>, String> {
    let right = pop_optional_integer_for_comparison(stack)?;
    let left = pop_optional_integer_for_comparison(stack)?;
    Ok(match (left, right) {
        (Some(left), Some(right)) => Some((left, right)),
        (None, None) => Some((0, 0)), // null == 0 in Neo N3 comparison semantics
        _ => None,
    })
}

pub(crate) fn pop_optional_integer_for_comparison(
    stack: &mut Vec<StackValue>,
) -> Result<Option<i64>, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(Some(value)),
        Some(StackValue::BigInteger(_)) => Err("expected integer on stack".to_string()),
        Some(StackValue::ByteString(_)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Boolean(value)) => Ok(Some(if value { 1 } else { 0 })),
        Some(StackValue::Pointer(_)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Array(..)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Struct(..)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Map(..)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Buffer(_, _)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Interop(_)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Iterator(_)) => Err("expected integer on stack".to_string()),
        Some(StackValue::Null) => Ok(None),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn pop_shift_count(stack: &mut Vec<StackValue>) -> Result<i64, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(value),
        Some(StackValue::Boolean(value)) => Ok(if value { 1 } else { 0 }),
        Some(StackValue::ByteString(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::BigInteger(value)) => decode_signed_le_bytes(&value),
        Some(StackValue::Null) => Ok(0),
        Some(StackValue::Buffer(_, bytes)) => decode_signed_le_bytes(&bytes),
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
        Some(StackValue::Null) => Ok(0),
        Some(StackValue::Pointer(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Array(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Struct(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Map(..)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Buffer(_, bytes)) => decode_signed_le_bytes(&bytes),
        Some(StackValue::Interop(_)) => Err("expected integer-compatible value".to_string()),
        Some(StackValue::Iterator(_)) => Err("expected integer-compatible value".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn pop_shift_value(stack: &mut Vec<StackValue>) -> Result<ShiftValue, String> {
    match stack.pop() {
        Some(StackValue::Integer(value)) => Ok(ShiftValue::Integer(value)),
        Some(StackValue::Boolean(value)) => Ok(ShiftValue::Integer(if value { 1 } else { 0 })),
        Some(StackValue::ByteString(value)) => {
            Ok(ShiftValue::ByteString(decode_signed_le_bytes(&value)?))
        }
        Some(StackValue::BigInteger(value)) => {
            Ok(ShiftValue::ByteString(decode_signed_le_bytes(&value)?))
        }
        Some(StackValue::Null) => Ok(ShiftValue::Integer(0)),
        Some(StackValue::Pointer(_)) => Err("expected integer-compatible shift value".to_string()),
        Some(StackValue::Array(..)) => Err("expected integer-compatible shift value".to_string()),
        Some(StackValue::Struct(..)) => Err("expected integer-compatible shift value".to_string()),
        Some(StackValue::Map(..)) => Err("expected integer-compatible shift value".to_string()),
        Some(StackValue::Buffer(_, bytes)) => {
            Ok(ShiftValue::ByteString(decode_signed_le_bytes(&bytes)?))
        }
        Some(StackValue::Interop(_)) => Err("expected integer-compatible shift value".to_string()),
        Some(StackValue::Iterator(_)) => Err("expected integer-compatible shift value".to_string()),
        None => Err("stack underflow".to_string()),
    }
}

pub(crate) fn num_equal(left: &StackValue, right: &StackValue) -> Result<bool, String> {
    match (left, right) {
        (StackValue::ByteString(left), StackValue::ByteString(right)) => Ok(left == right),
        (
            StackValue::Integer(_) | StackValue::Boolean(_) | StackValue::Null,
            StackValue::Integer(_) | StackValue::Boolean(_) | StackValue::Null,
        ) => Ok(integer_value_for_equality(left)? == integer_value_for_equality(right)?),
        _ => Err("NUMEQUAL expects primitive numeric or byte string values".to_string()),
    }
}

pub(crate) fn integer_value_for_equality(value: &StackValue) -> Result<i64, String> {
    match value {
        StackValue::Integer(value) => Ok(*value),
        StackValue::Boolean(value) => Ok(if *value { 1 } else { 0 }),
        StackValue::Null => Ok(0),
        _ => Err("expected integer-compatible value".to_string()),
    }
}

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
        0x21 => Ok(StackValue::Integer(numeric_value(&value)?)),
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

pub(crate) fn apply_abi_stack(
    stack: &mut Vec<StackValue>,
    abi_stack: Vec<neo_riscv_abi::StackValue>,
    ids: &mut CompoundIds,
) {
    let imported = abi_stack
        .into_iter()
        .map(|item| ids.import_abi(item))
        .collect::<Vec<_>>();
    let retired = core::mem::replace(stack, imported);
    core::mem::forget(retired);
}

pub(crate) fn invoke_syscall<H: SyscallProvider>(
    host: &mut H,
    api: u32,
    ip: usize,
    stack: &mut Vec<StackValue>,
    ids: &mut CompoundIds,
) -> Result<(), String> {
    let mut abi_stack = to_abi_stack(stack);
    match host.syscall(api, ip, &mut abi_stack) {
        Ok(()) => {
            apply_abi_stack(stack, abi_stack, ids);
            Ok(())
        }
        Err(e) => {
            // Leak abi_stack on error to avoid talc free-list corruption on RISC-V/PolkaVM
            core::mem::forget(abi_stack);
            Err(e)
        }
    }
}

pub(crate) fn invoke_callt<H: SyscallProvider>(
    host: &mut H,
    token: u16,
    ip: usize,
    stack: &mut Vec<StackValue>,
    ids: &mut CompoundIds,
) -> Result<(), String> {
    let mut abi_stack = to_abi_stack(stack);
    match host.callt(token, ip, &mut abi_stack) {
        Ok(()) => {
            apply_abi_stack(stack, abi_stack, ids);
            Ok(())
        }
        Err(e) => {
            // Leak abi_stack on error to avoid talc free-list corruption on RISC-V/PolkaVM
            core::mem::forget(abi_stack);
            Err(e)
        }
    }
}

pub(crate) fn numeric_value(value: &StackValue) -> Result<i64, String> {
    match value {
        StackValue::Integer(value) => Ok(*value),
        StackValue::Boolean(value) => Ok(if *value { 1 } else { 0 }),
        StackValue::ByteString(bytes) => decode_signed_le_bytes(bytes),
        StackValue::BigInteger(bytes) => decode_signed_le_bytes(bytes),
        StackValue::Buffer(_, bytes) => decode_signed_le_bytes(bytes),
        StackValue::Null => Ok(0),
        _ => Err("expected numeric-compatible value".to_string()),
    }
}

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
    if bytes.len() > 8 {
        return Err("integer-compatible byte string too large".to_string());
    }

    let sign_extend = if bytes.last().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0x00
    };
    let mut buffer = [sign_extend; 8];
    buffer[..bytes.len()].copy_from_slice(bytes);
    Ok(i64::from_le_bytes(buffer))
}

pub(crate) fn shift_left(value: i64, shift: u32) -> Result<i64, String> {
    if shift == 0 {
        return Ok(value);
    }
    if shift >= 64 {
        return if value == 0 {
            Ok(0)
        } else {
            Err("integer overflow for SHL".to_string())
        };
    }

    value
        .checked_shl(shift)
        .ok_or_else(|| "integer overflow for SHL".to_string())
}

pub(crate) enum ShiftValue {
    Integer(i64),
    ByteString(i64),
}

impl ShiftValue {
    pub(crate) fn shift_left(self, shift: u32) -> Result<StackValue, String> {
        let value = match self {
            ShiftValue::Integer(value) | ShiftValue::ByteString(value) => shift_left(value, shift)?,
        };
        Ok(match self {
            ShiftValue::Integer(_) => StackValue::Integer(value),
            ShiftValue::ByteString(_) => StackValue::ByteString(encode_integer(value)),
        })
    }

    pub(crate) fn shift_right(self, shift: u32) -> StackValue {
        let value = match self {
            ShiftValue::Integer(value) | ShiftValue::ByteString(value) => {
                if shift >= 64 {
                    if value < 0 {
                        -1
                    } else {
                        0
                    }
                } else {
                    value >> shift
                }
            }
        };
        match self {
            ShiftValue::Integer(_) => StackValue::Integer(value),
            ShiftValue::ByteString(_) => StackValue::ByteString(encode_integer(value)),
        }
    }
}

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
    const MAX_INTEGER_SIZE: usize = 32;
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

    let mut result: i128 = 1;
    let mut power = i128::from(base);
    let modulus = i128::from(modulus);
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

pub(crate) fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }

    let mut x0 = value;
    let mut x1 = (x0 + value / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + value / x0) / 2;
    }
    x0
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
        StackValue::Pointer(_) | StackValue::Array(..) | StackValue::Struct(..) | StackValue::Map(..)
        | StackValue::Interop(_) | StackValue::Iterator(_) => {
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
    match (left, right) {
        (StackValue::Boolean(l), StackValue::Boolean(r)) => Ok(StackValue::Integer(i64::from(op(
            if *l { 1 } else { 0 },
            if *r { 1 } else { 0 },
        )))),
        (StackValue::Integer(l), StackValue::Integer(r)) => {
            Ok(StackValue::Integer(bytes_to_integer(
                &bitwise_signed_bytes(&encode_integer(*l), &encode_integer(*r), op)?,
            )))
        }
        (StackValue::BigInteger(l), StackValue::BigInteger(r)) => {
            Ok(bigint_or_integer(bitwise_signed_bytes(l, r, op)?))
        }
        (StackValue::ByteString(l), StackValue::ByteString(r)) => {
            Ok(bigint_or_integer(bitwise_signed_bytes(l, r, op)?))
        }
        (StackValue::Null, StackValue::Null) => Ok(StackValue::Integer(i64::from(op(0, 0)))),
        _ => Err("bitwise op expects matching integer, boolean, or byte string types".to_string()),
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
