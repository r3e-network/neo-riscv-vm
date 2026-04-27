extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{fast_codec, ExecutionResult, VmState};

#[inline]
pub fn encode_execution_result(result: &Result<ExecutionResult, String>) -> Vec<u8> {
    match result {
        Ok(execution) => {
            let stack_bytes = fast_codec::encode_stack(&execution.stack);
            let fault_bytes = execution
                .fault_message
                .as_ref()
                .map(|message| message.as_bytes())
                .unwrap_or_default();

            let mut out = Vec::with_capacity(
                1 + 8 + 1 + 1 + 4 + fault_bytes.len() + 4 + stack_bytes.len() + 1 + 4,
            );
            out.push(0);
            out.extend_from_slice(&execution.fee_consumed_pico.to_le_bytes());
            out.push(match execution.state {
                VmState::Halt => 0,
                VmState::Fault => 1,
            });
            out.push(u8::from(execution.fault_message.is_some()));
            out.extend_from_slice(&(fault_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(fault_bytes);
            out.extend_from_slice(&(stack_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&stack_bytes);
            // Trailing fault_ip: always emit the has-flag byte so the codec is
            // forward-compatible. A value of None encodes as the single flag byte;
            // Some(ip) appends the u32 LE.
            out.push(u8::from(execution.fault_ip.is_some()));
            if let Some(ip) = execution.fault_ip {
                out.extend_from_slice(&ip.to_le_bytes());
            }
            // Trailing fault_locals: flag byte, then u32 LE length, then raw bytes when present.
            if let Some(ref bytes) = execution.fault_locals {
                out.push(1);
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            } else {
                out.push(0);
            }
            out
        }
        Err(message) => {
            let bytes = message.as_bytes();
            let mut out = Vec::with_capacity(1 + 4 + bytes.len());
            out.push(1);
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
            out
        }
    }
}

pub fn decode_execution_result(bytes: &[u8]) -> Result<Result<ExecutionResult, String>, String> {
    let mut cursor = Cursor::new(bytes);
    match cursor.read_u8()? {
        0 => {
            let fee = cursor.read_i64()?;
            let state = match cursor.read_u8()? {
                0 => VmState::Halt,
                1 => VmState::Fault,
                _ => return Err("invalid vm state tag".to_string()),
            };
            let has_fault = cursor.read_u8()? != 0;
            let fault_len = cursor.read_u32()? as usize;
            let fault_bytes = cursor.read_exact(fault_len)?;
            let fault_message = if has_fault {
                Some(
                    String::from_utf8(fault_bytes.to_vec())
                        .map_err(|_| "invalid utf-8 fault message".to_string())?,
                )
            } else {
                None
            };
            let stack_len = cursor.read_u32()? as usize;
            let stack_bytes = cursor.read_exact(stack_len)?;
            let stack = fast_codec::decode_stack(stack_bytes).map_err(|e| e.to_string())?;
            // Trailing fault_ip is optional for forward-compat with older guest payloads
            // that predate this field. If absent, treat as None.
            let fault_ip = if cursor.is_eof() {
                None
            } else {
                let has_fault_ip = cursor.read_u8()? != 0;
                if has_fault_ip {
                    Some(cursor.read_u32()?)
                } else {
                    None
                }
            };
            // Trailing fault_locals is optional for forward-compat.
            let fault_locals = if cursor.is_eof() {
                None
            } else {
                let has_fault_locals = cursor.read_u8()? != 0;
                if has_fault_locals {
                    let len = cursor.read_u32()? as usize;
                    Some(cursor.read_exact(len)?.to_vec())
                } else {
                    None
                }
            };
            cursor.expect_eof()?;
            Ok(Ok(ExecutionResult {
                fee_consumed_pico: fee,
                state,
                stack,
                fault_message,
                fault_ip,
                fault_locals,
            }))
        }
        1 => {
            let error_len = cursor.read_u32()? as usize;
            let error = String::from_utf8(cursor.read_exact(error_len)?.to_vec())
                .map_err(|_| "invalid utf-8 error payload".to_string())?;
            cursor.expect_eof()?;
            Ok(Err(error))
        }
        _ => Err("invalid execution result tag".to_string()),
    }
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
        let byte = self.bytes[self.offset];
        self.offset += 1;
        Ok(byte)
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
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
            Err("trailing bytes after execution result".to_string())
        }
    }

    fn is_eof(&self) -> bool {
        self.offset >= self.bytes.len()
    }
}
