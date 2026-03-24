extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::StackValue;

#[inline]
pub fn encode_stack_result(result: &Result<Vec<StackValue>, String>) -> Vec<u8> {
    let capacity = match result {
        Ok(stack) => 1 + 4 + stack.len() * 16,
        Err(msg) => 1 + 4 + msg.len(),
    };
    let mut out = Vec::with_capacity(capacity);
    match result {
        Ok(stack) => {
            out.push(0);
            encode_u32(stack.len() as u32, &mut out);
            for item in stack {
                encode_stack_value(item, &mut out);
            }
        }
        Err(message) => {
            out.push(1);
            encode_bytes(message.as_bytes(), &mut out);
        }
    }
    out
}

pub fn decode_stack_result_into(
    bytes: &[u8],
    stack: &mut Vec<StackValue>,
) -> Result<Result<(), String>, String> {
    let mut cursor = Cursor::new(bytes);
    let tag = cursor.read_u8()?;
    let value = match tag {
        0 => {
            let len = cursor.read_u32()? as usize;
            stack.clear();
            stack.reserve(len);
            for _ in 0..len {
                stack.push(decode_stack_value(&mut cursor)?);
            }
            Ok(())
        }
        1 => {
            let message = cursor.read_bytes()?.to_vec();
            Err(String::from_utf8(message)
                .map_err(|_| "invalid utf-8 error payload".to_string())?)
        }
        _ => return Err("invalid stack result tag".to_string()),
    };
    cursor.expect_eof()?;
    Ok(value)
}

pub fn decode_stack_result(bytes: &[u8]) -> Result<Result<Vec<StackValue>, String>, String> {
    let mut cursor = Cursor::new(bytes);
    let tag = cursor.read_u8()?;
    let value = match tag {
        0 => {
            let len = cursor.read_u32()? as usize;
            let mut stack = Vec::with_capacity(len);
            for _ in 0..len {
                stack.push(decode_stack_value(&mut cursor)?);
            }
            Ok(stack)
        }
        1 => {
            let message = cursor.read_bytes()?.to_vec();
            Err(String::from_utf8(message)
                .map_err(|_| "invalid utf-8 error payload".to_string())?)
        }
        _ => return Err("invalid stack result tag".to_string()),
    };
    cursor.expect_eof()?;
    Ok(value)
}

#[inline]
fn encode_stack_value(value: &StackValue, out: &mut Vec<u8>) {
    match value {
        StackValue::Integer(value) => {
            out.push(0);
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&value.to_le_bytes());
            out.extend_from_slice(&buf);
        }
        StackValue::BigInteger(bytes) => {
            out.push(1);
            encode_bytes(bytes, out);
        }
        StackValue::ByteString(bytes) => {
            out.push(2);
            encode_bytes(bytes, out);
        }
        StackValue::Boolean(value) => {
            out.push(3);
            out.push(u8::from(*value));
        }
        StackValue::Array(items) => {
            out.push(4);
            encode_u32(items.len() as u32, out);
            for item in items {
                encode_stack_value(item, out);
            }
        }
        StackValue::Struct(items) => {
            out.push(5);
            encode_u32(items.len() as u32, out);
            for item in items {
                encode_stack_value(item, out);
            }
        }
        StackValue::Map(entries) => {
            out.push(6);
            encode_u32(entries.len() as u32, out);
            for (key, value) in entries {
                encode_stack_value(key, out);
                encode_stack_value(value, out);
            }
        }
        StackValue::Interop(handle) => {
            out.push(7);
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&handle.to_le_bytes());
            out.extend_from_slice(&buf);
        }
        StackValue::Iterator(handle) => {
            out.push(8);
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&handle.to_le_bytes());
            out.extend_from_slice(&buf);
        }
        StackValue::Null => out.push(9),
        StackValue::Pointer(value) => {
            out.push(10);
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&value.to_le_bytes());
            out.extend_from_slice(&buf);
        }
    }
}

#[inline]
fn decode_stack_value(cursor: &mut Cursor<'_>) -> Result<StackValue, String> {
    match cursor.read_u8()? {
        0 => Ok(StackValue::Integer(cursor.read_i64()?)),
        1 => Ok(StackValue::BigInteger(cursor.read_bytes()?.to_vec())),
        2 => Ok(StackValue::ByteString(cursor.read_bytes()?.to_vec())),
        3 => Ok(StackValue::Boolean(cursor.read_u8()? != 0)),
        4 => {
            let len = cursor.read_u32()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_stack_value(cursor)?);
            }
            Ok(StackValue::Array(items))
        }
        5 => {
            let len = cursor.read_u32()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_stack_value(cursor)?);
            }
            Ok(StackValue::Struct(items))
        }
        6 => {
            let len = cursor.read_u32()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                let key = decode_stack_value(cursor)?;
                let value = decode_stack_value(cursor)?;
                items.push((key, value));
            }
            Ok(StackValue::Map(items))
        }
        7 => Ok(StackValue::Interop(cursor.read_u64()?)),
        8 => Ok(StackValue::Iterator(cursor.read_u64()?)),
        9 => Ok(StackValue::Null),
        10 => Ok(StackValue::Pointer(cursor.read_i64()?)),
        _ => Err("invalid stack value tag".to_string()),
    }
}

#[inline]
fn encode_u32(value: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&buf);
}

#[inline]
fn encode_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    let mut len_buf = [0u8; 4];
    len_buf.copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&len_buf);
    out.extend_from_slice(bytes);
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.offset >= self.bytes.len() {
            return Err("unexpected end of input".to_string());
        }
        let value = self.bytes[self.offset];
        self.offset += 1;
        Ok(value)
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        Ok(self.read_u64()? as i64)
    }

    fn read_bytes(&mut self) -> Result<&'a [u8], String> {
        let len = self.read_u32()? as usize;
        self.read_exact(len)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "offset overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("unexpected end of input".to_string());
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn expect_eof(&self) -> Result<(), String> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err("trailing bytes in payload".to_string())
        }
    }
}
