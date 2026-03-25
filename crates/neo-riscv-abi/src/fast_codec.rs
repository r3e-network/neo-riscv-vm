//! Fast binary codec for StackValue serialization
//!
//! Custom binary format optimized for Neo RISC-V VM stack operations.
//! Replaces postcard with zero-copy, type-tagged encoding.

extern crate alloc;
use alloc::vec::Vec;
use crate::StackValue;

// Type tags (1 byte each)
const TAG_INTEGER: u8 = 0x01;
const TAG_BIGINTEGER: u8 = 0x02;
const TAG_BYTESTRING: u8 = 0x03;
const TAG_BOOLEAN: u8 = 0x04;
const TAG_ARRAY: u8 = 0x05;
const TAG_STRUCT: u8 = 0x06;
const TAG_MAP: u8 = 0x07;
const TAG_INTEROP: u8 = 0x08;
const TAG_ITERATOR: u8 = 0x09;
const TAG_NULL: u8 = 0x0A;
const TAG_POINTER: u8 = 0x0B;

/// Encode stack to binary format
#[inline]
pub fn encode_stack(stack: &[StackValue]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(stack.len() * 32);

    // Stack length (4 bytes)
    buf.extend_from_slice(&(stack.len() as u32).to_le_bytes());

    for item in stack {
        encode_value(item, &mut buf);
    }

    buf
}

/// Encode stack into a pre-allocated slice (for no_std guest)
#[inline]
pub fn encode_stack_to_slice<'a>(
    stack: &[StackValue],
    buf: &'a mut [u8],
) -> Result<&'a mut [u8], &'static str> {
    // Stack length (4 bytes)
    if buf.len() < 4 {
        return Err("buffer too small for stack length");
    }
    let len_bytes = (stack.len() as u32).to_le_bytes();
    buf[0..4].copy_from_slice(&len_bytes);
    let mut pos = 4;

    for item in stack {
        pos = encode_value_to_slice(item, buf, pos)?;
    }

    Ok(&mut buf[..pos])
}

/// Decode stack from binary format
#[inline]
pub fn decode_stack(bytes: &[u8]) -> Result<Vec<StackValue>, &'static str> {
    if bytes.len() < 4 {
        return Err("truncated stack length");
    }

    let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let mut stack = Vec::with_capacity(len);
    let mut pos = 4;

    for _ in 0..len {
        let (value, consumed) = decode_value(&bytes[pos..])?;
        stack.push(value);
        pos += consumed;
    }

    Ok(stack)
}

#[inline]
fn encode_value(value: &StackValue, buf: &mut Vec<u8>) {
    match value {
        StackValue::Integer(i) => {
            buf.push(TAG_INTEGER);
            buf.extend_from_slice(&i.to_le_bytes());
        }
        StackValue::BigInteger(b) => {
            buf.push(TAG_BIGINTEGER);
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        StackValue::ByteString(b) => {
            buf.push(TAG_BYTESTRING);
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        StackValue::Boolean(b) => {
            buf.push(TAG_BOOLEAN);
            buf.push(if *b { 1 } else { 0 });
        }
        StackValue::Array(items) => {
            buf.push(TAG_ARRAY);
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                encode_value(item, buf);
            }
        }
        StackValue::Struct(items) => {
            buf.push(TAG_STRUCT);
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                encode_value(item, buf);
            }
        }
        StackValue::Map(pairs) => {
            buf.push(TAG_MAP);
            buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
            for (k, v) in pairs {
                encode_value(k, buf);
                encode_value(v, buf);
            }
        }
        StackValue::Interop(h) => {
            buf.push(TAG_INTEROP);
            buf.extend_from_slice(&h.to_le_bytes());
        }
        StackValue::Iterator(h) => {
            buf.push(TAG_ITERATOR);
            buf.extend_from_slice(&h.to_le_bytes());
        }
        StackValue::Null => {
            buf.push(TAG_NULL);
        }
        StackValue::Pointer(p) => {
            buf.push(TAG_POINTER);
            buf.extend_from_slice(&p.to_le_bytes());
        }
    }
}

#[inline]
fn encode_value_to_slice(value: &StackValue, buf: &mut [u8], mut pos: usize) -> Result<usize, &'static str> {
    match value {
        StackValue::Integer(i) => {
            if buf.len() < pos + 9 { return Err("buffer too small"); }
            buf[pos] = TAG_INTEGER;
            buf[pos+1..pos+9].copy_from_slice(&i.to_le_bytes());
            Ok(pos + 9)
        }
        StackValue::BigInteger(b) => {
            if buf.len() < pos + 5 + b.len() { return Err("buffer too small"); }
            buf[pos] = TAG_BIGINTEGER;
            buf[pos+1..pos+5].copy_from_slice(&(b.len() as u32).to_le_bytes());
            buf[pos+5..pos+5+b.len()].copy_from_slice(b);
            Ok(pos + 5 + b.len())
        }
        StackValue::ByteString(b) => {
            if buf.len() < pos + 5 + b.len() { return Err("buffer too small"); }
            buf[pos] = TAG_BYTESTRING;
            buf[pos+1..pos+5].copy_from_slice(&(b.len() as u32).to_le_bytes());
            buf[pos+5..pos+5+b.len()].copy_from_slice(b);
            Ok(pos + 5 + b.len())
        }
        StackValue::Boolean(b) => {
            if buf.len() < pos + 2 { return Err("buffer too small"); }
            buf[pos] = TAG_BOOLEAN;
            buf[pos+1] = if *b { 1 } else { 0 };
            Ok(pos + 2)
        }
        StackValue::Array(items) => {
            if buf.len() < pos + 5 { return Err("buffer too small"); }
            buf[pos] = TAG_ARRAY;
            buf[pos+1..pos+5].copy_from_slice(&(items.len() as u32).to_le_bytes());
            pos += 5;
            for item in items {
                pos = encode_value_to_slice(item, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Struct(items) => {
            if buf.len() < pos + 5 { return Err("buffer too small"); }
            buf[pos] = TAG_STRUCT;
            buf[pos+1..pos+5].copy_from_slice(&(items.len() as u32).to_le_bytes());
            pos += 5;
            for item in items {
                pos = encode_value_to_slice(item, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Map(pairs) => {
            if buf.len() < pos + 5 { return Err("buffer too small"); }
            buf[pos] = TAG_MAP;
            buf[pos+1..pos+5].copy_from_slice(&(pairs.len() as u32).to_le_bytes());
            pos += 5;
            for (k, v) in pairs {
                pos = encode_value_to_slice(k, buf, pos)?;
                pos = encode_value_to_slice(v, buf, pos)?;
            }
            Ok(pos)
        }
        StackValue::Interop(h) => {
            if buf.len() < pos + 9 { return Err("buffer too small"); }
            buf[pos] = TAG_INTEROP;
            buf[pos+1..pos+9].copy_from_slice(&h.to_le_bytes());
            Ok(pos + 9)
        }
        StackValue::Iterator(h) => {
            if buf.len() < pos + 9 { return Err("buffer too small"); }
            buf[pos] = TAG_ITERATOR;
            buf[pos+1..pos+9].copy_from_slice(&h.to_le_bytes());
            Ok(pos + 9)
        }
        StackValue::Null => {
            if buf.len() < pos + 1 { return Err("buffer too small"); }
            buf[pos] = TAG_NULL;
            Ok(pos + 1)
        }
        StackValue::Pointer(p) => {
            if buf.len() < pos + 9 { return Err("buffer too small"); }
            buf[pos] = TAG_POINTER;
            buf[pos+1..pos+9].copy_from_slice(&p.to_le_bytes());
            Ok(pos + 9)
        }
    }
}

#[inline]
fn decode_value(bytes: &[u8]) -> Result<(StackValue, usize), &'static str> {
    if bytes.is_empty() {
        return Err("empty buffer");
    }

    let tag = bytes[0];
    let mut pos = 1;

    let value = match tag {
        TAG_INTEGER => {
            if bytes.len() < pos + 8 {
                return Err("truncated integer");
            }
            let val = i64::from_le_bytes([
                bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3],
                bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7],
            ]);
            pos += 8;
            StackValue::Integer(val)
        }
        TAG_BIGINTEGER => {
            if bytes.len() < pos + 4 {
                return Err("truncated biginteger length");
            }
            let len = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]) as usize;
            pos += 4;
            if bytes.len() < pos + len {
                return Err("truncated biginteger data");
            }
            let data = bytes[pos..pos+len].to_vec();
            pos += len;
            StackValue::BigInteger(data)
        }
        TAG_BYTESTRING => {
            if bytes.len() < pos + 4 {
                return Err("truncated bytestring length");
            }
            let len = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]) as usize;
            pos += 4;
            if bytes.len() < pos + len {
                return Err("truncated bytestring data");
            }
            let data = bytes[pos..pos+len].to_vec();
            pos += len;
            StackValue::ByteString(data)
        }
        TAG_BOOLEAN => {
            if bytes.len() < pos + 1 {
                return Err("truncated boolean");
            }
            let val = bytes[pos] != 0;
            pos += 1;
            StackValue::Boolean(val)
        }
        TAG_ARRAY => {
            if bytes.len() < pos + 4 {
                return Err("truncated array length");
            }
            let len = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]) as usize;
            pos += 4;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                let (item, consumed) = decode_value(&bytes[pos..])?;
                items.push(item);
                pos += consumed;
            }
            StackValue::Array(items)
        }
        TAG_STRUCT => {
            if bytes.len() < pos + 4 {
                return Err("truncated struct length");
            }
            let len = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]) as usize;
            pos += 4;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                let (item, consumed) = decode_value(&bytes[pos..])?;
                items.push(item);
                pos += consumed;
            }
            StackValue::Struct(items)
        }
        TAG_MAP => {
            if bytes.len() < pos + 4 {
                return Err("truncated map length");
            }
            let len = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]) as usize;
            pos += 4;
            let mut pairs = Vec::with_capacity(len);
            for _ in 0..len {
                let (k, k_consumed) = decode_value(&bytes[pos..])?;
                pos += k_consumed;
                let (v, v_consumed) = decode_value(&bytes[pos..])?;
                pos += v_consumed;
                pairs.push((k, v));
            }
            StackValue::Map(pairs)
        }
        TAG_INTEROP => {
            if bytes.len() < pos + 8 {
                return Err("truncated interop");
            }
            let val = u64::from_le_bytes([
                bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3],
                bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7],
            ]);
            pos += 8;
            StackValue::Interop(val)
        }
        TAG_ITERATOR => {
            if bytes.len() < pos + 8 {
                return Err("truncated iterator");
            }
            let val = u64::from_le_bytes([
                bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3],
                bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7],
            ]);
            pos += 8;
            StackValue::Iterator(val)
        }
        TAG_NULL => StackValue::Null,
        TAG_POINTER => {
            if bytes.len() < pos + 8 {
                return Err("truncated pointer");
            }
            let val = i64::from_le_bytes([
                bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3],
                bytes[pos+4], bytes[pos+5], bytes[pos+6], bytes[pos+7],
            ]);
            pos += 8;
            StackValue::Pointer(val)
        }
        _ => return Err("invalid tag"),
    };

    Ok((value, pos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn roundtrip_integer() {
        let stack = vec![StackValue::Integer(42), StackValue::Integer(-100)];
        let encoded = encode_stack(&stack);
        let decoded = decode_stack(&encoded).unwrap();
        assert_eq!(stack, decoded);
    }

    #[test]
    fn roundtrip_bytestring() {
        let stack = vec![StackValue::ByteString(vec![1, 2, 3, 4])];
        let encoded = encode_stack(&stack);
        let decoded = decode_stack(&encoded).unwrap();
        assert_eq!(stack, decoded);
    }

    #[test]
    fn roundtrip_array() {
        let stack = vec![StackValue::Array(vec![
            StackValue::Integer(1),
            StackValue::Boolean(true),
            StackValue::Null,
        ])];
        let encoded = encode_stack(&stack);
        let decoded = decode_stack(&encoded).unwrap();
        assert_eq!(stack, decoded);
    }

    #[test]
    fn roundtrip_map() {
        let stack = vec![StackValue::Map(vec![
            (StackValue::Integer(1), StackValue::ByteString(vec![0xAA])),
            (StackValue::Boolean(false), StackValue::Null),
        ])];
        let encoded = encode_stack(&stack);
        let decoded = decode_stack(&encoded).unwrap();
        assert_eq!(stack, decoded);
    }
}
