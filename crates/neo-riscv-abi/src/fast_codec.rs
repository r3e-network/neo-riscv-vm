//! Fast binary codec for stack serialization
//!
//! This module provides a custom binary format optimized for speed.
//! It's designed to replace postcard for better performance.

extern crate alloc;

use alloc::vec::Vec;
use crate::StackValue;

/// Encode a stack value to bytes (fast path for common types)
#[inline(always)]
pub fn encode_stack_value_fast(value: &StackValue, out: &mut Vec<u8>) {
    match value {
        // Fast path: Integer (most common)
        StackValue::Integer(val) => {
            out.push(0);
            out.extend_from_slice(&val.to_le_bytes());
        }
        // Fast path: Boolean
        StackValue::Boolean(val) => {
            out.push(1);
            out.push(if *val { 1 } else { 0 });
        }
        // Fast path: Null
        StackValue::Null => {
            out.push(2);
        }
        // ByteString (common for data)
        StackValue::ByteString(bytes) => {
            out.push(3);
            encode_bytes_fast(bytes, out);
        }
        // Array
        StackValue::Array(items) => {
            out.push(4);
            encode_u32_fast(items.len() as u32, out);
            for item in items {
                encode_stack_value_fast(item, out);
            }
        }
        // Struct
        StackValue::Struct(items) => {
            out.push(5);
            encode_u32_fast(items.len() as u32, out);
            for item in items {
                encode_stack_value_fast(item, out);
            }
        }
        // Map
        StackValue::Map(entries) => {
            out.push(6);
            encode_u32_fast(entries.len() as u32, out);
            for (key, val) in entries {
                encode_stack_value_fast(key, out);
                encode_stack_value_fast(val, out);
            }
        }
        // Interop handle
        StackValue::Interop(handle) => {
            out.push(7);
            out.extend_from_slice(&handle.to_le_bytes());
        }
        // Iterator handle
        StackValue::Iterator(handle) => {
            out.push(8);
            out.extend_from_slice(&handle.to_le_bytes());
        }
        // BigInteger
        StackValue::BigInteger(bytes) => {
            out.push(9);
            encode_bytes_fast(bytes, out);
        }
        // Pointer
        StackValue::Pointer(val) => {
            out.push(10);
            out.extend_from_slice(&val.to_le_bytes());
        }
    }
}

/// Decode a stack value from bytes
#[inline(always)]
pub fn decode_stack_value_fast(cursor: &mut FastCursor<'_>) -> Result<StackValue, &'static str> {
    let tag = cursor.read_u8()?;
    match tag {
        0 => Ok(StackValue::Integer(cursor.read_i64()?)),
        1 => Ok(StackValue::Boolean(cursor.read_u8()? != 0)),
        2 => Ok(StackValue::Null),
        3 => Ok(StackValue::ByteString(cursor.read_bytes()?.to_vec())),
        4 => {
            let len = cursor.read_u32()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_stack_value_fast(cursor)?);
            }
            Ok(StackValue::Array(items))
        }
        5 => {
            let len = cursor.read_u32()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(decode_stack_value_fast(cursor)?);
            }
            Ok(StackValue::Struct(items))
        }
        6 => {
            let len = cursor.read_u32()? as usize;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                let key = decode_stack_value_fast(cursor)?;
                let val = decode_stack_value_fast(cursor)?;
                entries.push((key, val));
            }
            Ok(StackValue::Map(entries))
        }
        7 => Ok(StackValue::Interop(cursor.read_u64()?)),
        8 => Ok(StackValue::Iterator(cursor.read_u64()?)),
        9 => Ok(StackValue::BigInteger(cursor.read_bytes()?.to_vec())),
        10 => Ok(StackValue::Pointer(cursor.read_i64()?)),
        _ => Err("invalid tag"),
    }
}

/// Encode a full stack
#[inline]
pub fn encode_stack(stack: &[StackValue]) -> Vec<u8> {
    let capacity = 4 + stack.len() * 16; // Estimate
    let mut out = Vec::with_capacity(capacity);
    encode_u32_fast(stack.len() as u32, &mut out);
    for item in stack {
        encode_stack_value_fast(item, &mut out);
    }
    out
}

/// Decode a full stack
#[inline]
pub fn decode_stack(bytes: &[u8]) -> Result<Vec<StackValue>, &'static str> {
    let mut cursor = FastCursor::new(bytes);
    let len = cursor.read_u32()? as usize;
    let mut stack = Vec::with_capacity(len);
    for _ in 0..len {
        stack.push(decode_stack_value_fast(&mut cursor)?);
    }
    Ok(stack)
}

/// Encode a stack to a pre-allocated buffer
#[inline]
pub fn encode_stack_to_slice(stack: &[StackValue], buf: &mut [u8]) -> Result<usize, &'static str> {
    let mut writer = SliceWriter::new(buf);
    writer.write_u32(stack.len() as u32)?;
    for item in stack {
        encode_stack_value_to_writer(item, &mut writer)?;
    }
    Ok(writer.position())
}

#[inline(always)]
fn encode_stack_value_to_writer(value: &StackValue, writer: &mut SliceWriter<'_>) -> Result<(), &'static str> {
    match value {
        StackValue::Integer(val) => {
            writer.write_u8(0)?;
            writer.write_bytes(&val.to_le_bytes())?;
        }
        StackValue::Boolean(val) => {
            writer.write_u8(1)?;
            writer.write_u8(if *val { 1 } else { 0 })?;
        }
        StackValue::Null => {
            writer.write_u8(2)?;
        }
        StackValue::ByteString(bytes) => {
            writer.write_u8(3)?;
            writer.write_u32(bytes.len() as u32)?;
            writer.write_bytes(bytes)?;
        }
        StackValue::Array(items) => {
            writer.write_u8(4)?;
            writer.write_u32(items.len() as u32)?;
            for item in items {
                encode_stack_value_to_writer(item, writer)?;
            }
        }
        StackValue::Struct(items) => {
            writer.write_u8(5)?;
            writer.write_u32(items.len() as u32)?;
            for item in items {
                encode_stack_value_to_writer(item, writer)?;
            }
        }
        StackValue::Map(entries) => {
            writer.write_u8(6)?;
            writer.write_u32(entries.len() as u32)?;
            for (key, val) in entries {
                encode_stack_value_to_writer(key, writer)?;
                encode_stack_value_to_writer(val, writer)?;
            }
        }
        StackValue::Interop(handle) => {
            writer.write_u8(7)?;
            writer.write_bytes(&handle.to_le_bytes())?;
        }
        StackValue::Iterator(handle) => {
            writer.write_u8(8)?;
            writer.write_bytes(&handle.to_le_bytes())?;
        }
        StackValue::BigInteger(bytes) => {
            writer.write_u8(9)?;
            writer.write_u32(bytes.len() as u32)?;
            writer.write_bytes(bytes)?;
        }
        StackValue::Pointer(val) => {
            writer.write_u8(10)?;
            writer.write_bytes(&val.to_le_bytes())?;
        }
    }
    Ok(())
}

#[inline(always)]
fn encode_u32_fast(value: u32, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[inline(always)]
fn encode_bytes_fast(bytes: &[u8], out: &mut Vec<u8>) {
    encode_u32_fast(bytes.len() as u32, out);
    out.extend_from_slice(bytes);
}

/// Fast cursor for reading bytes
pub struct FastCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> FastCursor<'a> {
    #[inline(always)]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    #[inline(always)]
    pub fn read_u8(&mut self) -> Result<u8, &'static str> {
        if self.offset >= self.bytes.len() {
            return Err("eof");
        }
        let val = self.bytes[self.offset];
        self.offset += 1;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u32(&mut self) -> Result<u32, &'static str> {
        if self.offset + 4 > self.bytes.len() {
            return Err("eof");
        }
        let val = u32::from_le_bytes([
            self.bytes[self.offset],
            self.bytes[self.offset + 1],
            self.bytes[self.offset + 2],
            self.bytes[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_u64(&mut self) -> Result<u64, &'static str> {
        if self.offset + 8 > self.bytes.len() {
            return Err("eof");
        }
        let val = u64::from_le_bytes([
            self.bytes[self.offset],
            self.bytes[self.offset + 1],
            self.bytes[self.offset + 2],
            self.bytes[self.offset + 3],
            self.bytes[self.offset + 4],
            self.bytes[self.offset + 5],
            self.bytes[self.offset + 6],
            self.bytes[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(val)
    }

    #[inline(always)]
    pub fn read_i64(&mut self) -> Result<i64, &'static str> {
        Ok(self.read_u64()? as i64)
    }

    #[inline(always)]
    pub fn read_bytes(&mut self) -> Result<&'a [u8], &'static str> {
        let len = self.read_u32()? as usize;
        if self.offset + len > self.bytes.len() {
            return Err("eof");
        }
        let slice = &self.bytes[self.offset..self.offset + len];
        self.offset += len;
        Ok(slice)
    }
}

/// Writer for encoding to a fixed-size buffer
pub struct SliceWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceWriter<'a> {
    #[inline(always)]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    #[inline(always)]
    pub fn position(&self) -> usize {
        self.pos
    }

    #[inline(always)]
    pub fn write_u8(&mut self, val: u8) -> Result<(), &'static str> {
        if self.pos >= self.buf.len() {
            return Err("overflow");
        }
        self.buf[self.pos] = val;
        self.pos += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn write_u32(&mut self, val: u32) -> Result<(), &'static str> {
        if self.pos + 4 > self.buf.len() {
            return Err("overflow");
        }
        self.buf[self.pos..self.pos + 4].copy_from_slice(&val.to_le_bytes());
        self.pos += 4;
        Ok(())
    }

    #[inline(always)]
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if self.pos + bytes.len() > self.buf.len() {
            return Err("overflow");
        }
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }
}
