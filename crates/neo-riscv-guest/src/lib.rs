#![no_std]

extern crate alloc;

mod helpers;
mod opcodes;
mod runtime_types;

use crate::helpers::*;

use crate::opcodes::*;
use crate::runtime_types::{
    find_affected_indices, propagate_update, to_abi_stack, CompoundIds, StackValue,
};

#[derive(Debug, Clone)]
struct TryFrame {
    catch_ip: usize,
    finally_ip: usize,
    caught: bool,
    in_finally: bool,
    end_ip: usize,
}

/// Fixed-capacity stack for TryFrames — avoids heap allocation to prevent
/// PolkaVM bump allocator corruption during host_call round-trips.
const MAX_TRY_NESTING: usize = 16;

struct TryStack {
    frames: [core::mem::MaybeUninit<TryFrame>; MAX_TRY_NESTING],
    len: usize,
}

impl TryStack {
    #[inline]
    fn new() -> Self {
        Self {
            // Safety: MaybeUninit does not require initialization
            frames: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    fn push(&mut self, frame: TryFrame) -> Result<(), String> {
        if self.len >= MAX_TRY_NESTING {
            return Err("TRY nesting exceeds maximum depth".to_string());
        }
        self.frames[self.len] = core::mem::MaybeUninit::new(frame);
        self.len += 1;
        Ok(())
    }

    #[inline]
    fn pop(&mut self) -> Option<TryFrame> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // Safety: frames[self.len] was previously initialized by push()
        Some(unsafe { self.frames[self.len].assume_init_read() })
    }

    #[inline]
    fn last_mut(&mut self) -> Option<&mut TryFrame> {
        if self.len == 0 {
            return None;
        }
        // Safety: frames[self.len - 1] was previously initialized by push()
        Some(unsafe { self.frames[self.len - 1].assume_init_mut() })
    }

    /// Find the last uncaught frame (iterating in reverse)
    fn find_uncaught_mut(&mut self) -> Option<&mut TryFrame> {
        for i in (0..self.len).rev() {
            // Safety: frames[i] was previously initialized by push()
            let frame = unsafe { &mut *self.frames[i].as_mut_ptr() };
            if !frame.caught {
                return Some(frame);
            }
        }
        None
    }
}

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use neo_riscv_abi::{ExecutionResult, StackValue as AbiStackValue, VmState};

pub fn interpret(script: &[u8]) -> Result<ExecutionResult, String> {
    let mut host = NoSyscalls;
    interpret_with_stack_and_syscalls_at(script, Vec::new(), 0, &mut host)
}

pub trait SyscallProvider {
    fn on_instruction(&mut self, _opcode: u8) -> Result<(), String> {
        Ok(())
    }

    fn syscall(
        &mut self,
        api: u32,
        ip: usize,
        stack: &mut Vec<AbiStackValue>,
    ) -> Result<(), String>;

    /// Handle CALLT opcode. The default encodes the token with a distinctive
    /// marker so the host can distinguish CALLT from regular SYSCALL.
    fn callt(
        &mut self,
        token: u16,
        ip: usize,
        stack: &mut Vec<AbiStackValue>,
    ) -> Result<(), String> {
        // Encode: CALLT_MARKER_HI in upper 16 bits + token in low 16 bits.
        // Host checks (api >> 16) == CALLT_MARKER_HI to detect CALLT calls.
        self.syscall(CALLT_MARKER | token as u32, ip, stack)
    }
}

/// Marker for CALLT tokens sent through the syscall channel.
/// Upper 16 bits = 0x4354 ("CT"), lower 16 bits = token_id.
/// This pattern is checked via (api >> 16) == 0x4354.
pub const CALLT_MARKER: u32 = 0x4354_0000;
pub const CALLT_MARKER_HI: u16 = 0x4354;

pub fn interpret_with_syscalls<H: SyscallProvider>(
    script: &[u8],
    host: &mut H,
) -> Result<ExecutionResult, String> {
    interpret_with_stack_and_syscalls_at(script, Vec::new(), 0, host)
}

pub fn interpret_with_stack_and_syscalls<H: SyscallProvider>(
    script: &[u8],
    initial_stack: Vec<AbiStackValue>,
    host: &mut H,
) -> Result<ExecutionResult, String> {
    interpret_with_stack_and_syscalls_at(script, initial_stack, 0, host)
}

pub fn interpret_with_stack_and_syscalls_at<H: SyscallProvider>(
    script: &[u8],
    initial_stack: Vec<AbiStackValue>,
    initial_ip: usize,
    host: &mut H,
) -> Result<ExecutionResult, String> {
    if initial_ip > script.len() {
        return Err("initial instruction pointer out of bounds".to_string());
    }
    let mut ids = CompoundIds::default();
    let mut stack = initial_stack
        .into_iter()
        .map(|item| ids.import_abi(item))
        .collect::<Vec<_>>();
    let mut ip = initial_ip;
    let mut locals: Vec<StackValue> = Vec::with_capacity(16);
    let mut static_fields: Vec<StackValue> = Vec::with_capacity(16);
    let mut locals_initialized = false;
    let mut static_fields_initialized = false;
    let mut alt_stack: Vec<StackValue> = Vec::with_capacity(16);
    let mut try_frames = TryStack::new();
    let mut call_stack: Vec<(usize, Vec<StackValue>, bool)> = Vec::with_capacity(16);
    let mut pending_error: Option<String> = None;

    'main_loop: loop {
        if pending_error.is_some() {
            // Find the topmost un-caught frame
            if let Some(frame) = try_frames.find_uncaught_mut() {
                frame.caught = true;
                // Save frame fields into locals BEFORE any allocating operations.
                // Under PolkaVM's bump allocator, host-call round-trips can reset the
                // allocator offset, causing subsequent allocations to overwrite the
                // TryFrame backing buffer.  Copying into stack locals avoids the stale read.
                let saved_catch_ip = frame.catch_ip;
                let saved_finally_ip = frame.finally_ip;
                // NeoVM: error goes to catch first, then finally
                if saved_catch_ip != 0 {
                    let msg = pending_error.take().unwrap();
                    stack.push(StackValue::ByteString(msg.into_bytes()));
                    ip = saved_catch_ip;
                } else if saved_finally_ip != 0 {
                    frame.in_finally = true;
                    ip = saved_finally_ip;
                    // Keep pending_error for re-throw after ENDFINALLY
                } else {
                    pending_error = None;
                    continue;
                }
                continue;
            } else {
                return Err(pending_error.take().unwrap());
            }
        }

        // Stack overflow check: NeoVM faults when stack exceeds 2048 items.
        if stack.len() > 2048 {
            return Err("stack overflow".to_string());
        }

        if ip >= script.len() {
            break;
        }

        let opcode = script[ip];
        host.on_instruction(opcode)?;
        match opcode {
            // =============================================================================
            // PUSH OPCODES (0x00-0x20)
            // =============================================================================
            PUSHINT8 => {
                if ip + 2 > script.len() {
                    return Err("truncated PUSHINT8 operand".to_string());
                }
                let value = i8::from_le_bytes([script[ip + 1]]) as i64;
                stack.push(StackValue::Integer(value));
                ip += 2;
                continue;
            }
            PUSHINT16 => {
                if ip + 3 > script.len() {
                    return Err("truncated PUSHINT16 operand".to_string());
                }
                let value = i16::from_le_bytes([script[ip + 1], script[ip + 2]]) as i64;
                stack.push(StackValue::Integer(value));
                ip += 3;
                continue;
            }
            PUSHINT32 => {
                if ip + 5 > script.len() {
                    return Err("truncated PUSHINT32 operand".to_string());
                }
                let value = i32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]) as i64;
                stack.push(StackValue::Integer(value));
                ip += 5;
                continue;
            }
            PUSHINT64 => {
                if ip + 9 > script.len() {
                    return Err("truncated PUSHINT64 operand".to_string());
                }
                let value = i64::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                    script[ip + 5],
                    script[ip + 6],
                    script[ip + 7],
                    script[ip + 8],
                ]);
                stack.push(StackValue::Integer(value));
                ip += 9;
                continue;
            }
            PUSHINT128 => {
                if ip + 17 > script.len() {
                    return Err("truncated PUSHINT128 operand".to_string());
                }
                stack.push(StackValue::BigInteger(trim_le_bytes_slice(
                    &script[ip + 1..ip + 17],
                )));
                ip += 17;
                continue;
            }
            PUSHINT256 => {
                if ip + 33 > script.len() {
                    return Err("truncated PUSHINT256 operand".to_string());
                }
                stack.push(StackValue::BigInteger(trim_le_bytes_slice(
                    &script[ip + 1..ip + 33],
                )));
                ip += 33;
                continue;
            }
            PUSHT => {
                stack.push(StackValue::Boolean(true));
                ip += 1;
                continue;
            }
            PUSHF => {
                stack.push(StackValue::Boolean(false));
                ip += 1;
                continue;
            }
            PUSHNULL => stack.push(StackValue::Null),
            PUSHDATA1 => {
                if ip + 2 > script.len() {
                    return Err("truncated PUSHDATA1 length".to_string());
                }
                let len = script[ip + 1] as usize;
                let start = ip + 2;
                let end = start + len;
                if end > script.len() {
                    return Err("truncated PUSHDATA1 payload".to_string());
                }
                stack.push(StackValue::ByteString(script[start..end].to_vec()));
                ip = end;
                continue;
            }
            PUSHDATA2 => {
                if ip + 3 > script.len() {
                    return Err("truncated PUSHDATA2 length".to_string());
                }
                let len = u16::from_le_bytes([script[ip + 1], script[ip + 2]]) as usize;
                let start = ip + 3;
                let end = start + len;
                if end > script.len() {
                    return Err("truncated PUSHDATA2 payload".to_string());
                }
                stack.push(StackValue::ByteString(script[start..end].to_vec()));
                ip = end;
                continue;
            }
            PUSHDATA4 => {
                if ip + 5 > script.len() {
                    return Err("truncated PUSHDATA4 length".to_string());
                }
                let raw_len = u32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]);
                // NeoVM: negative (high bit set) or oversized lengths are invalid
                if raw_len > 0x0010_0000 {
                    return Err("PUSHDATA4 length exceeds maximum".to_string());
                }
                let len = raw_len as usize;
                let start = ip + 5;
                let end = start
                    .checked_add(len)
                    .ok_or_else(|| "PUSHDATA4 length overflow".to_string())?;
                if end > script.len() {
                    return Err("truncated PUSHDATA4 payload".to_string());
                }
                stack.push(StackValue::ByteString(script[start..end].to_vec()));
                ip = end;
                continue;
            }
            PUSHM1 => stack.push(StackValue::Integer(-1)),
            TOALTSTACK => {
                let value = pop_item(&mut stack)?;
                alt_stack.push(value);
            }
            FROMALTSTACK => {
                let value = alt_stack
                    .pop()
                    .ok_or_else(|| "alt stack underflow".to_string())?;
                stack.push(value);
            }
            PUSH0 => stack.push(StackValue::Integer(0)),
            PUSH1..=PUSH16 => stack.push(StackValue::Integer(i64::from(opcode - PUSH0))),
            // =============================================================================
            // FLOW CONTROL OPCODES (0x21-0x40)
            // =============================================================================
            JMP | JMP_L => {
                let is_long = opcode == JMP_L;
                let (offset, _advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMP",
                )?;
                ip = compute_jump_target_offset(ip, offset, script.len(), "JMP")?;
                continue;
            }
            JMPIF | JMPIF_L => {
                let is_long = opcode == JMPIF_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPIF",
                )?;
                let condition = pop_boolean(&mut stack)?;
                if condition {
                    ip = compute_jump_target_offset(ip, offset, script.len(), "JMPIF")?;
                    continue;
                }
                ip += advance;
                continue;
            }
            JMPEQ | JMPEQ_L => {
                let is_long = opcode == JMPEQ_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPEQ",
                )?;
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                if vm_equal(&left, &right) {
                    ip = compute_jump_target_offset(ip, offset, script.len(), "JMPEQ")?;
                    continue;
                }
                ip += advance;
                continue;
            }
            ABORT => {
                // ABORT is uncatchable — always FAULT
                return Err("ABORT".to_string());
            }
            ASSERT => {
                let value = pop_boolean(&mut stack)?;
                if !value {
                    // ASSERT is uncatchable — always FAULT
                    return Err("ASSERT failed".to_string());
                }
            }
            SYSCALL => {
                if ip + 5 > script.len() {
                    return Err("truncated SYSCALL operand".to_string());
                }

                let api = u32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]);
                if let Err(e) = invoke_syscall(host, api, ip, &mut stack, &mut ids) {
                    if try_frames.is_empty() {
                        return Err(e);
                    }
                    pending_error = Some(e);
                    continue;
                }
                ip += 5;
                continue;
            }
            CALLT => {
                if ip + 3 > script.len() {
                    return Err("truncated CALLT operand".to_string());
                }
                let token = u16::from_le_bytes([script[ip + 1], script[ip + 2]]);
                if let Err(e) = invoke_callt(host, token, ip, &mut stack, &mut ids) {
                    if try_frames.is_empty() {
                        return Err(e);
                    }
                    pending_error = Some(e);
                    continue;
                }
                ip += 3;
                continue;
            }
            NOP => {}
            // =============================================================================
            // STACK OPERATIONS (0x43-0x54)
            // =============================================================================
            DEPTH => {
                stack.push(StackValue::Integer(stack.len() as i64));
            }
            DROP => {
                pop_item(&mut stack)?;
            }
            DUP => {
                let value = peek_item(&stack)?;
                stack.push(value);
            }
            SWAP => {
                if stack.len() < 2 {
                    return Err("stack underflow for SWAP".to_string());
                }
                let last = stack.len() - 1;
                stack.swap(last, last - 1);
            }
            // =============================================================================
            // SLOT OPERATIONS (0x56-0x87)
            // =============================================================================
            INITSSLOT => {
                if ip + 2 > script.len() {
                    return Err("truncated INITSSLOT operand".to_string());
                }
                let static_count = script[ip + 1] as usize;
                if static_count == 0 {
                    return Err("INITSSLOT with 0 items is not allowed".to_string());
                }
                if static_fields_initialized {
                    return Err("static fields already initialized".to_string());
                }
                static_fields = vec![StackValue::Null; static_count];
                static_fields_initialized = true;
                ip += 2;
                continue;
            }
            INITSLOT => {
                if ip + 3 > script.len() {
                    return Err("truncated INITSLOT operands".to_string());
                }
                let local_count = script[ip + 1] as usize;
                let arg_count = script[ip + 2] as usize;
                // NeoVM: INITSLOT with 0 args AND 0 locals is invalid
                if local_count == 0 && arg_count == 0 {
                    return Err("INITSLOT with 0 args and 0 locals is not allowed".to_string());
                }
                if locals_initialized {
                    return Err("slots already initialized".to_string());
                }
                // Neo N3: INITSLOT allocates locals[arg_count..arg_count+local_count-1] as Null,
                // and pops arg_count items from the eval stack into locals[0..arg_count-1].
                let mut new_locals = Vec::with_capacity(arg_count + local_count);
                for _ in 0..arg_count {
                    new_locals.push(
                        stack
                            .pop()
                            .ok_or_else(|| "stack underflow for INITSLOT args".to_string())?,
                    );
                }
                // NeoVM: args are stored in pop order (top of stack = arg0)
                new_locals.resize(arg_count + local_count, StackValue::Null);
                locals = new_locals;
                locals_initialized = true;
                ip += 3;
                continue;
            }
            LDSFLD0..=LDSFLD6 => {
                let index = (opcode - LDSFLD0) as usize;
                let value = static_fields
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid static field index".to_string())?;
                stack.push(value);
            }
            LDSFLD => {
                if ip + 2 > script.len() {
                    return Err("truncated LDSFLD operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = static_fields
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid static field index".to_string())?;
                stack.push(value);
                ip += 2;
                continue;
            }
            LDLOC0..=LDLOC6 => {
                let index = (opcode - LDLOC0) as usize;
                let value = locals
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid local index".to_string())?;
                stack.push(value);
            }
            LDLOC => {
                if ip + 2 > script.len() {
                    return Err("truncated LDLOC operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = locals
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid local index".to_string())?;
                stack.push(value);
                ip += 2;
                continue;
            }
            STLOC0..=STLOC6 => {
                let index = (opcode - STLOC0) as usize;
                let value = pop_item(&mut stack)?;
                let slot = locals
                    .get_mut(index)
                    .ok_or_else(|| "invalid local index".to_string())?;
                *slot = value;
            }
            STSFLD0..=STSFLD6 => {
                let index = (opcode - STSFLD0) as usize;
                let value = pop_item(&mut stack)?;
                let slot = static_fields
                    .get_mut(index)
                    .ok_or_else(|| "invalid static field index".to_string())?;
                *slot = value;
            }
            STSFLD => {
                if ip + 2 > script.len() {
                    return Err("truncated STSFLD operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = pop_item(&mut stack)?;
                let slot = static_fields
                    .get_mut(index)
                    .ok_or_else(|| "invalid static field index".to_string())?;
                *slot = value;
                ip += 2;
                continue;
            }
            STLOC => {
                if ip + 2 > script.len() {
                    return Err("truncated STLOC operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = pop_item(&mut stack)?;
                let slot = locals
                    .get_mut(index)
                    .ok_or_else(|| "invalid local index".to_string())?;
                *slot = value;
                ip += 2;
                continue;
            }
            // =============================================================================
            // SPLICE OPERATIONS (0x88-0x8e)
            // =============================================================================
            CAT => {
                let right_item = pop_item(&mut stack)?;
                let left_item = pop_item(&mut stack)?;
                let left_bytes = helpers::stack_item_to_bytes(left_item)?;
                let right_bytes = helpers::stack_item_to_bytes(right_item)?;
                let mut result_bytes = left_bytes;
                result_bytes.extend_from_slice(&right_bytes);
                // NeoVM: CAT result must not exceed max item size (1024*1024)
                const MAX_ITEM_SIZE: usize = 1024 * 1024;
                if result_bytes.len() > MAX_ITEM_SIZE {
                    return Err("CAT result exceeds max item size".to_string());
                }
                // NeoVM: CAT always produces a Buffer
                stack.push(ids.buffer(result_bytes));
            }
            LEFT => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for LEFT".to_string());
                }
                let bytes = pop_bytes(&mut stack)?;
                let count = count as usize;
                if count > bytes.len() {
                    return Err("count out of range for LEFT".to_string());
                }
                stack.push(StackValue::ByteString(bytes[..count].to_vec()));
            }
            NEWBUFFER => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for NEWBUFFER".to_string());
                }
                if count > 1_048_576 {
                    return Err("buffer size exceeds MaxItemSize (1MB)".to_string());
                }
                stack.push(ids.buffer(vec![0u8; count as usize]));
            }
            // =============================================================================
            // BITWISE LOGIC OPERATIONS (0x90-0x98)
            // =============================================================================
            INVERT => {
                let value = pop_item(&mut stack)?;
                match value {
                    StackValue::Integer(v) => stack.push(StackValue::Integer(!v)),
                    StackValue::BigInteger(v) => {
                        let inverted: Vec<u8> = v.iter().map(|b| !b).collect();
                        stack.push(StackValue::BigInteger(trim_le_bytes(inverted)));
                    }
                    StackValue::ByteString(v) => {
                        // NeoVM: ByteString is implicitly convertible to Integer for bitwise ops
                        let inverted: Vec<u8> = v.iter().map(|b| !b).collect();
                        stack.push(StackValue::BigInteger(trim_le_bytes(inverted)));
                    }
                    StackValue::Boolean(v) => {
                        stack.push(StackValue::Integer(if v { -2 } else { -1 }))
                    }
                    StackValue::Null => stack.push(StackValue::Integer(-1)),
                    _ => return Err("INVERT expects an integer or boolean".to_string()),
                }
            }
            AND => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(bitwise_result(&left, &right, |l, r| l & r)?);
            }
            OR => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(bitwise_result(&left, &right, |l, r| l | r)?);
            }
            XOR => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(bitwise_result(&left, &right, |l, r| l ^ r)?);
            }
            NUMEQUAL => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(StackValue::Boolean(num_equal(&left, &right)?));
            }
            NUMNOTEQUAL => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(StackValue::Boolean(!num_equal(&left, &right)?));
            }
            EQUAL => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(StackValue::Boolean(vm_equal(&left, &right)));
            }
            NOTEQUAL => {
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                stack.push(StackValue::Boolean(!vm_equal(&left, &right)));
            }
            LT => {
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                stack.push(StackValue::Boolean(
                    matches!(comparison, Some((left, right)) if left < right),
                ));
            }
            LE => {
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                stack.push(StackValue::Boolean(
                    matches!(comparison, Some((left, right)) if left <= right),
                ));
            }
            GT => {
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                stack.push(StackValue::Boolean(
                    matches!(comparison, Some((left, right)) if left > right),
                ));
            }
            GE => {
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                stack.push(StackValue::Boolean(
                    matches!(comparison, Some((left, right)) if left >= right),
                ));
            }
            // =============================================================================
            // ARITHMETIC OPERATIONS (0x99-0xbb)
            // =============================================================================
            SIGN => {
                let value = pop_numeric_value(&mut stack)?;
                stack.push(StackValue::Integer(value.signum()));
            }
            ABS => {
                let value = pop_numeric_value(&mut stack)?;
                stack.push(StackValue::Integer(
                    value
                        .checked_abs()
                        .ok_or_else(|| "integer overflow for ABS".to_string())?,
                ));
            }
            NEGATE => {
                let value = pop_numeric_value(&mut stack)?;
                stack.push(StackValue::Integer(
                    value
                        .checked_neg()
                        .ok_or_else(|| "integer overflow for NEGATE".to_string())?,
                ));
            }
            ADD => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                let sum = left
                    .checked_add(right)
                    .ok_or_else(|| "integer overflow for ADD".to_string())?;
                stack.push(StackValue::Integer(sum));
            }
            INC => {
                let value = pop_integer(&mut stack)?;
                stack.push(StackValue::Integer(
                    value
                        .checked_add(1)
                        .ok_or_else(|| "integer overflow for INC".to_string())?,
                ));
            }
            SUB => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                let difference = left
                    .checked_sub(right)
                    .ok_or_else(|| "integer overflow for SUB".to_string())?;
                stack.push(StackValue::Integer(difference));
            }
            POW => {
                let exponent = pop_integer(&mut stack)?;
                if exponent < 0 {
                    return Err("negative exponent for POW".to_string());
                }
                let base = pop_integer(&mut stack)?;
                let mut result: i128 = 1;
                for _ in 0..(exponent as u64) {
                    result = result
                        .checked_mul(i128::from(base))
                        .ok_or_else(|| "integer overflow for POW".to_string())?;
                }
                stack.push(StackValue::Integer(
                    i64::try_from(result).map_err(|_| "integer overflow for POW".to_string())?,
                ));
            }
            SQRT => {
                let value = pop_numeric_value(&mut stack)?;
                if value < 0 {
                    return Err("negative value for SQRT".to_string());
                }
                stack.push(StackValue::Integer(integer_sqrt(value as u64) as i64));
            }
            MODMUL => {
                let modulus = pop_integer(&mut stack)?;
                if modulus == 0 {
                    return Err("division by zero for MODMUL".to_string());
                }
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                let result = ((left as i128) * (right as i128)) % (modulus as i128);
                stack.push(StackValue::Integer(
                    i64::try_from(result).map_err(|_| "integer overflow for MODMUL".to_string())?,
                ));
            }
            MODPOW => {
                let modulus = pop_integer(&mut stack)?;
                if modulus == 0 {
                    return Err("division by zero for MODPOW".to_string());
                }
                let exponent = pop_integer(&mut stack)?;
                let base = pop_integer(&mut stack)?;
                stack.push(StackValue::Integer(mod_pow(base, exponent, modulus)?));
            }
            SHL => {
                let shift = pop_shift_count(&mut stack)?;
                let value = pop_shift_value(&mut stack)?;
                if !(-256..=256).contains(&shift) {
                    return Err("shift count out of range for SHL".to_string());
                }
                if shift >= 0 {
                    stack.push(value.shift_left(shift as u32)?);
                } else {
                    stack.push(value.shift_right((-shift) as u32));
                }
            }
            SHR => {
                let shift = pop_shift_count(&mut stack)?;
                let value = pop_shift_value(&mut stack)?;
                if !(-256..=256).contains(&shift) {
                    return Err("shift count out of range for SHR".to_string());
                }
                if shift >= 0 {
                    stack.push(value.shift_right(shift as u32));
                } else {
                    stack.push(value.shift_left((-shift) as u32)?);
                }
            }
            NOT => {
                // NeoVM: NOT converts to boolean via integer path.
                // ByteString > 32 bytes cannot be converted to integer → FAULT.
                let item = pop_item(&mut stack)?;
                let b = item_to_boolean_strict(&item)?;
                stack.push(StackValue::Boolean(!b));
            }
            // =============================================================================
            // COMPOUND TYPE OPERATIONS (0xbe-0xd3)
            // =============================================================================
            PACKMAP => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for PACKMAP".to_string());
                }
                let count = count as usize;
                if stack.len() < count.saturating_mul(2) {
                    return Err("stack underflow for PACKMAP".to_string());
                }

                let mut pairs = Vec::with_capacity(count);
                for _ in 0..count {
                    let key = pop_item(&mut stack)?;
                    let value = pop_item(&mut stack)?;
                    pairs.push((key, value));
                }
                stack.push(ids.map(pairs));
            }
            PACKSTRUCT => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for PACKSTRUCT".to_string());
                }
                let count = count as usize;
                if stack.len() < count {
                    return Err("stack underflow for PACKSTRUCT".to_string());
                }

                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(pop_item(&mut stack)?);
                }
                let items = items
                    .into_iter()
                    .map(|item| ids.clone_struct_for_storage(&item))
                    .collect();
                stack.push(ids.r#struct(items));
            }
            PACK => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for PACK".to_string());
                }
                let count = count as usize;
                if stack.len() < count {
                    return Err("stack underflow for PACK".to_string());
                }

                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(pop_item(&mut stack)?);
                }
                let items = items
                    .into_iter()
                    .map(|item| ids.clone_struct_for_storage(&item))
                    .collect();
                stack.push(ids.array(items));
            }
            UNPACK => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(_, items) | StackValue::Struct(_, items) => {
                        let count = items.len() as i64;
                        for item in items.into_iter().rev() {
                            stack.push(item);
                        }
                        stack.push(StackValue::Integer(count));
                    }
                    StackValue::Map(_, items) => {
                        let count = items.len() as i64;
                        for (key, value) in items.into_iter().rev() {
                            stack.push(value);
                            stack.push(key);
                        }
                        stack.push(StackValue::Integer(count));
                    }
                    _ => return Err("UNPACK expects an array, struct, or map".to_string()),
                }
            }
            NEWARRAY0 => {
                stack.push(ids.array(Vec::new()));
            }
            NEWARRAY => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for NEWARRAY".to_string());
                }
                stack.push(ids.array(vec![StackValue::Null; count as usize]));
            }
            NEWARRAY_T => {
                if ip + 2 > script.len() {
                    return Err("truncated NEWARRAY_T type".to_string());
                }
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for NEWARRAY_T".to_string());
                }
                let kind = script[ip + 1];
                let default_value = match kind {
                    0x21 => StackValue::Integer(0),
                    0x28 => StackValue::ByteString(Vec::new()),
                    _ => StackValue::Null,
                };
                stack.push(ids.array(vec![default_value; count as usize]));
                ip += 2;
                continue;
            }
            NEWSTRUCT0 => {
                stack.push(ids.r#struct(Vec::new()));
            }
            NEWSTRUCT => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for NEWSTRUCT".to_string());
                }
                stack.push(ids.r#struct(vec![StackValue::Null; count as usize]));
            }
            NEWMAP => {
                stack.push(ids.map(Vec::new()));
            }
            SIZE => {
                let item = pop_item(&mut stack)?;
                let size = match item {
                    StackValue::ByteString(bytes) => bytes.len() as i64,
                    StackValue::Array(_, items) => items.len() as i64,
                    StackValue::Struct(_, items) => items.len() as i64,
                    StackValue::Map(_, items) => items.len() as i64,
                    StackValue::Null => 0,
                    StackValue::Buffer(_, bytes) => bytes.len() as i64,
                    StackValue::Integer(_)
                    | StackValue::BigInteger(_)
                    | StackValue::Boolean(_)
                    | StackValue::Pointer(_)
                    | StackValue::Interop(_)
                    | StackValue::Iterator(_) => {
                        return Err("SIZE expects a collection".to_string())
                    }
                };
                stack.push(StackValue::Integer(size));
            }
            HASKEY => {
                let key = pop_item(&mut stack)?;
                let item = pop_item(&mut stack)?;
                let has_key = match item {
                    StackValue::ByteString(bytes) => {
                        let index = integer_value_for_collection_index(&key)?;
                        index >= 0 && (index as usize) < bytes.len()
                    }
                    StackValue::Buffer(_, bytes) => {
                        let index = integer_value_for_collection_index(&key)?;
                        index >= 0 && (index as usize) < bytes.len()
                    }
                    StackValue::Array(_, items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        index >= 0 && (index as usize) < items.len()
                    }
                    StackValue::Struct(_, items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        index >= 0 && (index as usize) < items.len()
                    }
                    StackValue::Map(_, items) => {
                        validate_map_key(&key)?;
                        items
                            .iter()
                            .any(|(candidate, _)| primitive_key_equals(candidate, &key))
                    }
                    _ => return Err("HASKEY expects an array, buffer, or map".to_string()),
                };
                stack.push(StackValue::Boolean(has_key));
            }
            KEYS => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Map(_, items) => {
                        let len = items.len();
                        let keys: Vec<_> = {
                            let mut v = Vec::with_capacity(len);
                            v.extend(items.into_iter().map(|(key, _)| key));
                            v
                        };
                        stack.push(ids.array(keys));
                    }
                    _ => return Err("KEYS expects a map".to_string()),
                }
            }
            VALUES => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Map(_, items) => {
                        let values = items
                            .into_iter()
                            .map(|(_, value)| value)
                            .collect::<Vec<_>>();
                        stack.push(ids.array(values));
                    }
                    StackValue::Array(_, items) => {
                        stack.push(ids.array(items));
                    }
                    StackValue::Struct(_, items) => {
                        stack.push(ids.array(items));
                    }
                    _ => return Err("VALUES expects a map, array, or struct".to_string()),
                }
            }
            APPEND => {
                let value = pop_item(&mut stack)?;
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(id, mut items) => {
                        items.push(ids.clone_struct_for_storage(&value));
                        let updated = StackValue::Array(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Struct(id, mut items) => {
                        items.push(ids.clone_struct_for_storage(&value));
                        let updated = StackValue::Struct(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    _ => return Err("APPEND expects an array or struct".to_string()),
                }
            }
            PICKITEM => {
                let key_or_index = pop_item(&mut stack)?;
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Map(_, items) => {
                        // Map key can be any primitive type
                        validate_map_key(&key_or_index)?;
                        let value = items
                            .iter()
                            .find(|(candidate, _)| primitive_key_equals(candidate, &key_or_index))
                            .map(|(_, value)| value.clone())
                            .ok_or_else(|| "key not found for PICKITEM".to_string())?;
                        stack.push(value);
                    }
                    _ => {
                        // Array, Struct, Buffer, ByteString: key must be integer index
                        let index = match key_or_index {
                            StackValue::Integer(v) if v >= 0 => v as usize,
                            StackValue::Boolean(v) => {
                                if v {
                                    1
                                } else {
                                    0
                                }
                            }
                            StackValue::Null => 0,
                            _ => {
                                return Err(
                                    "PICKITEM index must be a non-negative integer".to_string()
                                )
                            }
                        };
                        match item {
                            StackValue::Array(_, items) | StackValue::Struct(_, items) => {
                                let value = items
                                    .get(index)
                                    .cloned()
                                    .ok_or_else(|| "index out of range for PICKITEM".to_string())?;
                                stack.push(value);
                            }
                            StackValue::Buffer(_, bytes) => {
                                let value = bytes
                                    .get(index)
                                    .copied()
                                    .ok_or_else(|| "index out of range for PICKITEM".to_string())?;
                                stack.push(StackValue::Integer(i64::from(value)));
                            }
                            StackValue::ByteString(bytes) => {
                                let value = bytes
                                    .get(index)
                                    .copied()
                                    .ok_or_else(|| "index out of range for PICKITEM".to_string())?;
                                stack.push(StackValue::Integer(i64::from(value)));
                            }
                            _ => {
                                return Err(
                                    "PICKITEM expects an array, map, or byte string".to_string()
                                )
                            }
                        }
                    }
                }
            }
            SETITEM => {
                let value = pop_item(&mut stack)?;
                let key = pop_item(&mut stack)?;
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::ByteString(_) => {
                        return Err(
                            "SETITEM expects a mutable buffer, array, struct, or map".to_string()
                        )
                    }
                    StackValue::Buffer(id, mut bytes) => {
                        let index = integer_value_for_collection_index(&key)?;
                        if index < 0 || (index as usize) >= bytes.len() {
                            return Err("index out of range for SETITEM".to_string());
                        }
                        let byte = match value {
                            StackValue::Integer(value) if (0..=255).contains(&value) => value as u8,
                            StackValue::ByteString(value) if value.len() == 1 => value[0],
                            _ => return Err("SETITEM on buffer expects a byte value".to_string()),
                        };
                        bytes[index as usize] = byte;
                        let updated = StackValue::Buffer(id, bytes);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Array(id, mut items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        if index < 0 || (index as usize) >= items.len() {
                            return Err("index out of range for SETITEM".to_string());
                        }
                        items[index as usize] = ids.clone_struct_for_storage(&value);
                        let updated = StackValue::Array(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Struct(id, mut items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        if index < 0 || (index as usize) >= items.len() {
                            return Err("index out of range for SETITEM".to_string());
                        }
                        items[index as usize] = ids.clone_struct_for_storage(&value);
                        let updated = StackValue::Struct(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Map(id, mut items) => {
                        validate_map_key(&key)?;
                        if let Some((_, existing)) = items
                            .iter_mut()
                            .find(|(candidate, _)| primitive_key_equals(candidate, &key))
                        {
                            *existing = ids.clone_struct_for_storage(&value);
                        } else {
                            items.push((key, ids.clone_struct_for_storage(&value)));
                        }
                        let updated = StackValue::Map(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    _ => return Err("SETITEM expects an array, buffer, or map".to_string()),
                }
            }
            REMOVE => {
                let key = pop_item(&mut stack)?;
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(id, mut items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        if index < 0 || (index as usize) >= items.len() {
                            return Err("index out of range for REMOVE".to_string());
                        }
                        items.remove(index as usize);
                        let updated = StackValue::Array(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Struct(id, mut items) => {
                        let index = integer_value_for_collection_index(&key)?;
                        if index < 0 || (index as usize) >= items.len() {
                            return Err("index out of range for REMOVE".to_string());
                        }
                        items.remove(index as usize);
                        let updated = StackValue::Struct(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    StackValue::Map(id, mut items) => {
                        validate_map_key(&key)?;
                        let index = items
                            .iter()
                            .position(|(candidate, _)| primitive_key_equals(candidate, &key))
                            .ok_or_else(|| "key not found for REMOVE".to_string())?;
                        items.remove(index);
                        let updated = StackValue::Map(id, items);
                        let affected = find_affected_indices(id, &stack);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            Some(&affected),
                        );
                    }
                    _ => return Err("REMOVE expects an array, struct, or map".to_string()),
                }
            }
            CLEARITEMS => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(id, _) => {
                        let updated = StackValue::Array(id, Vec::new());
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    StackValue::Struct(id, _) => {
                        let updated = StackValue::Struct(id, Vec::new());
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    StackValue::Map(id, _) => {
                        let updated = StackValue::Map(id, Vec::new());
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    StackValue::Buffer(id, _) => {
                        let updated = StackValue::Buffer(id, Vec::new());
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    _ => return Err("CLEARITEMS expects a compound value".to_string()),
                }
            }
            POPITEM => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(id, mut items) => {
                        let popped = items
                            .pop()
                            .ok_or_else(|| "POPITEM on empty array".to_string())?;
                        let updated = StackValue::Array(id, items);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                        stack.push(updated);
                        stack.push(popped);
                    }
                    StackValue::Struct(id, mut items) => {
                        let popped = items
                            .pop()
                            .ok_or_else(|| "POPITEM on empty struct".to_string())?;
                        let updated = StackValue::Struct(id, items);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                        stack.push(updated);
                        stack.push(popped);
                    }
                    StackValue::Map(id, mut entries) => {
                        let (key, value) = entries
                            .pop()
                            .ok_or_else(|| "POPITEM on empty map".to_string())?;
                        let updated = StackValue::Map(id, entries);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                        stack.push(updated);
                        stack.push(key);
                        stack.push(value);
                    }
                    StackValue::Buffer(id, mut bytes) => {
                        let byte = bytes
                            .pop()
                            .ok_or_else(|| "POPITEM on empty buffer".to_string())?;
                        let updated = StackValue::Buffer(id, bytes);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                        stack.push(updated);
                        stack.push(StackValue::Integer(byte as i64));
                    }
                    _ => return Err("POPITEM expects a compound value".to_string()),
                }
            }
            CONVERT => {
                if ip + 2 > script.len() {
                    return Err("truncated CONVERT operand".to_string());
                }
                let kind = script[ip + 1];
                let value = pop_item(&mut stack)?;
                let converted = convert_value(kind, value, &mut ids)?;
                stack.push(converted);
                ip += 2;
                continue;
            }
            REVERSEITEMS => {
                let item = pop_item(&mut stack)?;
                match item {
                    StackValue::Array(id, mut items) => {
                        items.reverse();
                        let updated = StackValue::Array(id, items);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    StackValue::Struct(id, mut items) => {
                        items.reverse();
                        let updated = StackValue::Struct(id, items);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    StackValue::Buffer(id, mut bytes) => {
                        bytes.reverse();
                        let updated = StackValue::Buffer(id, bytes);
                        propagate_update(
                            &updated,
                            &mut stack,
                            &mut locals,
                            &mut static_fields,
                            None,
                        );
                    }
                    _ => return Err("REVERSEITEMS expects an array, struct, or buffer".to_string()),
                }
            }
            // =============================================================================
            // EXCEPTION HANDLING OPCODES (0xf0-0xf1)
            // =============================================================================
            THROW => {
                let msg = pop_item(&mut stack)?;
                let err_msg = format!("THROW: {:?}", msg);
                if try_frames.is_empty() {
                    return Ok(ExecutionResult {
                        fee_consumed_pico: 0,
                        state: VmState::Fault,
                        stack: to_abi_stack(&stack),
                        fault_message: Some(err_msg),
                    });
                }
                pending_error = Some(err_msg);
                continue;
            }
            THROWIFNOT => {
                let condition = pop_boolean(&mut stack)?;
                if !condition {
                    let err_msg = "THROWIFNOT".to_string();
                    if try_frames.is_empty() {
                        return Ok(ExecutionResult {
                            fee_consumed_pico: 0,
                            state: VmState::Fault,
                            stack: to_abi_stack(&stack),
                            fault_message: Some(err_msg),
                        });
                    }
                    pending_error = Some(err_msg);
                    continue;
                }
            }
            MUL => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                let product = left
                    .checked_mul(right)
                    .ok_or_else(|| "integer overflow for MUL".to_string())?;
                stack.push(StackValue::Integer(product));
            }
            DIV => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                if right == 0 {
                    return Err("division by zero for DIV".to_string());
                }
                let quotient = left
                    .checked_div(right)
                    .ok_or_else(|| "integer overflow for DIV".to_string())?;
                stack.push(StackValue::Integer(quotient));
            }
            MOD => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                if right == 0 {
                    return Err("division by zero for MOD".to_string());
                }
                let remainder = left
                    .checked_rem(right)
                    .ok_or_else(|| "integer overflow for MOD".to_string())?;
                stack.push(StackValue::Integer(remainder));
            }
            DEC => {
                let value = pop_integer(&mut stack)?;
                stack.push(StackValue::Integer(
                    value
                        .checked_sub(1)
                        .ok_or_else(|| "integer overflow for DEC".to_string())?,
                ));
            }
            BOOLAND => {
                let right = pop_boolean(&mut stack)?;
                let left = pop_boolean(&mut stack)?;
                stack.push(StackValue::Boolean(left && right));
            }
            BOOLOR => {
                let right = pop_boolean(&mut stack)?;
                let left = pop_boolean(&mut stack)?;
                stack.push(StackValue::Boolean(left || right));
            }
            NZ => {
                let value = pop_numeric_value(&mut stack)?;
                stack.push(StackValue::Boolean(value != 0));
            }
            MIN => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                stack.push(StackValue::Integer(if left < right { left } else { right }));
            }
            MAX => {
                let right = pop_integer(&mut stack)?;
                let left = pop_integer(&mut stack)?;
                stack.push(StackValue::Integer(if left > right { left } else { right }));
            }
            WITHIN => {
                let upper = pop_integer(&mut stack)?;
                let lower = pop_integer(&mut stack)?;
                let value = pop_integer(&mut stack)?;
                stack.push(StackValue::Boolean(value >= lower && value < upper));
            }
            RIGHT => {
                let count = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for RIGHT".to_string());
                }
                let count = count as usize;
                let mut bytes = pop_bytes(&mut stack)?;
                if count > bytes.len() {
                    return Err("count out of range for RIGHT".to_string());
                }
                let start = bytes.len() - count;
                bytes = bytes[start..].to_vec();
                stack.push(StackValue::ByteString(bytes));
            }
            SUBSTR => {
                let count = pop_integer(&mut stack)?;
                let index = pop_integer(&mut stack)?;
                if count < 0 {
                    return Err("negative count for SUBSTR".to_string());
                }
                if index < 0 {
                    return Err("negative index for SUBSTR".to_string());
                }
                let index = index as usize;
                let count = count as usize;
                let bytes = pop_bytes(&mut stack)?;
                // NeoVM reference: error if index + count > length (NOT index > length)
                let end = index
                    .checked_add(count)
                    .ok_or_else(|| "SUBSTR index+count overflow".to_string())?;
                if end > bytes.len() {
                    return Err("index + count out of range for SUBSTR".to_string());
                }
                stack.push(StackValue::ByteString(bytes[index..end].to_vec()));
            }
            MEMCPY => {
                // NeoVM MEMCPY: stack = [dst, di, src, si, count] (count on top)
                let count = pop_integer(&mut stack)?;
                let si = pop_integer(&mut stack)?;
                let src_item = pop_item(&mut stack)?;
                let di = pop_integer(&mut stack)?;
                let dst_item = pop_item(&mut stack)?;
                if count < 0 || si < 0 || di < 0 {
                    return Err("negative index/count for MEMCPY".to_string());
                }
                let count = count as usize;
                let si = si as usize;
                let di = di as usize;
                let src_bytes = helpers::stack_item_to_bytes(src_item)?;
                let (dst_id, mut dst_bytes) = match dst_item {
                    StackValue::Buffer(id, bytes) => (id, bytes),
                    other => {
                        return Err(format!(
                            "MEMCPY expects buffer as destination, got {:?}",
                            other
                        ))
                    }
                };
                if si + count > src_bytes.len() || di + count > dst_bytes.len() {
                    return Err("MEMCPY out of bounds".to_string());
                }
                dst_bytes[di..di + count].copy_from_slice(&src_bytes[si..si + count]);
                let updated = StackValue::Buffer(dst_id, dst_bytes);
                propagate_update(&updated, &mut stack, &mut locals, &mut static_fields, None);
            }
            PUSHA => {
                if ip + 5 > script.len() {
                    return Err("truncated PUSHA operand".to_string());
                }
                let offset = i32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]) as i64;
                let target = ip as i64 + offset;
                if target < 0 || target as usize > script.len() {
                    return Err("PUSHA target out of bounds".to_string());
                }
                stack.push(StackValue::Pointer(target as usize));
                ip += 5;
                continue;
            }
            // =============================================================================
            // TYPE OPERATIONS (0xda-0xdb)
            // =============================================================================
            ISTYPE => {
                if ip + 2 > script.len() {
                    return Err("truncated ISTYPE operand".to_string());
                }
                let kind = script[ip + 1];
                let item = pop_item(&mut stack)?;
                // NeoVM StackItemType enum values
                let result = match kind {
                    0x00 => true,                                   // Any - always true
                    0x10 => matches!(item, StackValue::Pointer(_)), // Pointer
                    0x20 => matches!(item, StackValue::Boolean(_)), // Boolean
                    0x21 => matches!(item, StackValue::Integer(_) | StackValue::BigInteger(_)), // Integer
                    0x28 => matches!(item, StackValue::ByteString(_)), // ByteString
                    0x30 => matches!(item, StackValue::Buffer(_, _)),  // Buffer
                    0x40 => matches!(item, StackValue::Array(_, _)),   // Array
                    0x41 => matches!(item, StackValue::Struct(_, _)),  // Struct
                    0x48 => matches!(item, StackValue::Map(_, _)),     // Map
                    0x60 => matches!(item, StackValue::Interop(_)),    // InteropInterface
                    _ => return Err(format!("unsupported ISTYPE kind 0x{kind:02x}")),
                };
                stack.push(StackValue::Boolean(result));
                ip += 2;
                continue;
            }
            ISNULL => {
                let item = pop_item(&mut stack)?;
                stack.push(StackValue::Boolean(matches!(item, StackValue::Null)));
            }
            NIP => {
                if stack.len() < 2 {
                    return Err("stack underflow for NIP".to_string());
                }
                let x1 = stack.pop().expect("guarded by length check");
                stack.pop();
                stack.push(x1);
            }
            OVER => {
                if stack.len() < 2 {
                    return Err("stack underflow for OVER".to_string());
                }
                let x1 = stack[stack.len() - 2].clone();
                stack.push(x1);
            }
            PICK => {
                let n = pop_integer(&mut stack)?;
                if n < 0 {
                    return Err("negative index for PICK".to_string());
                }
                let n = n as usize;
                if n >= stack.len() {
                    return Err("index out of range for PICK".to_string());
                }
                let item = stack[stack.len() - 1 - n].clone();
                stack.push(item);
            }
            ROT => {
                // Rotate top 3 items: bottom moves to top, top and second shift down
                if stack.len() < 3 {
                    return Err("stack underflow for ROT".to_string());
                }
                let n = stack.len() - 1;
                stack.swap(n - 2, n - 1);
                stack.swap(n - 1, n);
            }
            ROLL => {
                let n = pop_integer(&mut stack)?;
                if n < 0 {
                    return Err("negative index for ROLL".to_string());
                }
                let n = n as usize;
                if n >= stack.len() {
                    return Err("index out of range for ROLL".to_string());
                }
                let idx = stack.len() - 1 - n;
                let item = stack.remove(idx);
                stack.push(item);
            }
            REVERSE3 => {
                // Reverse top 3 items: [a, b, c] where c is top → [c, b, a] where a is top
                if stack.len() < 3 {
                    return Err("stack underflow for REVERSE3".to_string());
                }
                let n = stack.len() - 1;
                stack.swap(n - 2, n);
            }
            REVERSE4 => {
                // Reverse top 4 items: [a, b, c, d] where d is top → [d, c, b, a] where a is top
                if stack.len() < 4 {
                    return Err("stack underflow for REVERSE4".to_string());
                }
                let n = stack.len() - 1;
                stack.swap(n - 3, n);
                stack.swap(n - 2, n - 1);
            }
            REVERSEN => {
                let n = pop_integer(&mut stack)?;
                if n < 0 {
                    return Err("negative count for REVERSEN".to_string());
                }
                let n = n as usize;
                if n > stack.len() {
                    return Err("REVERSEN count exceeds stack depth".to_string());
                }
                let start = stack.len() - n;
                stack[start..].reverse();
            }
            TUCK => {
                if stack.len() < 2 {
                    return Err("stack underflow for TUCK".to_string());
                }
                let x = stack[stack.len() - 1].clone();
                stack.insert(stack.len() - 2, x);
            }
            XDROP => {
                let n = pop_integer(&mut stack)?;
                if n < 0 {
                    return Err("negative index for XDROP".to_string());
                }
                let n = n as usize;
                if n >= stack.len() {
                    return Err("XDROP index out of range".to_string());
                }
                let idx = stack.len() - 1 - n;
                stack.remove(idx);
            }
            LDARG0..=LDARG6 => {
                let index = (opcode - LDARG0) as usize;
                let value = locals
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid argument index".to_string())?;
                stack.push(value);
            }
            LDARG => {
                if ip + 2 > script.len() {
                    return Err("truncated LDARG operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = locals
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "invalid argument index".to_string())?;
                stack.push(value);
                ip += 2;
                continue;
            }
            STARG0..=STARG6 => {
                let index = (opcode - STARG0) as usize;
                let value = pop_item(&mut stack)?;
                let slot = locals
                    .get_mut(index)
                    .ok_or_else(|| "invalid argument index".to_string())?;
                *slot = value;
            }
            STARG => {
                if ip + 2 > script.len() {
                    return Err("truncated STARG operand".to_string());
                }
                let index = script[ip + 1] as usize;
                let value = pop_item(&mut stack)?;
                let slot = locals
                    .get_mut(index)
                    .ok_or_else(|| "invalid argument index".to_string())?;
                *slot = value;
                ip += 2;
                continue;
            }
            CLEAR => {
                stack.clear();
            }
            CALL | CALL_L => {
                let is_long = opcode == CALL_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "CALL",
                )?;
                let return_ip = ip + advance;
                let saved_locals = core::mem::replace(&mut locals, Vec::with_capacity(16));
                let saved_init = locals_initialized;
                call_stack.push((return_ip, saved_locals, saved_init));
                locals_initialized = false;
                ip = compute_jump_target_offset(ip, offset, script.len(), "CALL")?;
                continue;
            }
            CALLA => {
                let item = pop_item(&mut stack)?;
                let offset = match item {
                    StackValue::Pointer(p) => p,
                    _ => return Err("CALLA expects a pointer".to_string()),
                };
                let return_ip = ip + 1;
                let saved_locals = core::mem::replace(&mut locals, Vec::with_capacity(16));
                let saved_init = locals_initialized;
                call_stack.push((return_ip, saved_locals, saved_init));
                locals_initialized = false;
                if offset > script.len() {
                    return Err("CALLA target out of bounds".to_string());
                }
                ip = offset;
                continue;
            }
            ABORTMSG => {
                let msg = pop_bytes(&mut stack)?;
                let msg_str = String::from_utf8_lossy(&msg);
                let err_msg = format!("ABORTMSG is executed. Reason: {}", msg_str);
                // ABORTMSG is uncatchable — always FAULT
                return Ok(ExecutionResult {
                    fee_consumed_pico: 0,
                    state: VmState::Fault,
                    stack: to_abi_stack(&stack),
                    fault_message: Some(err_msg),
                });
            }
            ASSERTMSG => {
                let msg = pop_bytes(&mut stack)?;
                let condition = pop_boolean(&mut stack)?;
                if !condition {
                    let msg_str = String::from_utf8_lossy(&msg);
                    let err_msg = format!(
                        "ASSERTMSG is executed with false result. Reason: {}",
                        msg_str
                    );
                    // ASSERTMSG is uncatchable — always FAULT
                    return Ok(ExecutionResult {
                        fee_consumed_pico: 0,
                        state: VmState::Fault,
                        stack: to_abi_stack(&stack),
                        fault_message: Some(err_msg),
                    });
                }
            }
            JMPIFNOT | JMPIFNOT_L => {
                let is_long = opcode == JMPIFNOT_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPIFNOT",
                )?;
                let condition = pop_boolean(&mut stack)?;
                if !condition {
                    ip = compute_jump_target_offset(ip, offset, script.len(), "JMPIFNOT")?;
                    continue;
                }
                ip += advance;
                continue;
            }
            JMPNE | JMPNE_L => {
                let is_long = opcode == JMPNE_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPNE",
                )?;
                let right = pop_item(&mut stack)?;
                let left = pop_item(&mut stack)?;
                if !vm_equal(&left, &right) {
                    ip = compute_jump_target_offset(ip, offset, script.len(), "JMPNE")?;
                    continue;
                }
                ip += advance;
                continue;
            }
            JMPGT | JMPGT_L => {
                let is_long = opcode == JMPGT_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPGT",
                )?;
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                if let Some((left, right)) = comparison {
                    if left > right {
                        ip = compute_jump_target_offset(ip, offset, script.len(), "JMPGT")?;
                        continue;
                    }
                }
                ip += advance;
                continue;
            }
            JMPGE | JMPGE_L => {
                let is_long = opcode == JMPGE_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPGE",
                )?;
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                if let Some((left, right)) = comparison {
                    if left >= right {
                        ip = compute_jump_target_offset(ip, offset, script.len(), "JMPGE")?;
                        continue;
                    }
                }
                ip += advance;
                continue;
            }
            JMPLT | JMPLT_L => {
                let is_long = opcode == JMPLT_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPLT",
                )?;
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                if let Some((left, right)) = comparison {
                    if left < right {
                        ip = compute_jump_target_offset(ip, offset, script.len(), "JMPLT")?;
                        continue;
                    }
                }
                ip += advance;
                continue;
            }
            JMPLE | JMPLE_L => {
                let is_long = opcode == JMPLE_L;
                let (offset, advance) = read_offset(
                    script,
                    ip,
                    if is_long {
                        &Offset::Long
                    } else {
                        &Offset::Short
                    },
                    "JMPLE",
                )?;
                let comparison = pop_integer_pair_allowing_null_false(&mut stack)?;
                if let Some((left, right)) = comparison {
                    if left <= right {
                        ip = compute_jump_target_offset(ip, offset, script.len(), "JMPLE")?;
                        continue;
                    }
                }
                ip += advance;
                continue;
            }
            RET => {
                if let Some((return_ip, saved_locals, saved_init)) = call_stack.pop() {
                    locals = saved_locals;
                    locals_initialized = saved_init;
                    ip = return_ip;
                    continue;
                }
                break 'main_loop;
            }
            TRY => {
                // NeoVM TRY: TRY catch_offset_i8, finally_offset_i8 (3 bytes total)
                if ip + 3 > script.len() {
                    pending_error = Some("truncated TRY operand".to_string());
                    continue;
                }
                let catch_offset = script[ip + 1] as i8;
                let finally_offset = script[ip + 2] as i8;
                let catch_ip = if catch_offset != 0 {
                    (ip as isize + catch_offset as isize) as usize
                } else {
                    0
                };
                let finally_ip = if finally_offset != 0 {
                    (ip as isize + finally_offset as isize) as usize
                } else {
                    0
                };
                if catch_ip > script.len() || finally_ip > script.len() {
                    pending_error = Some("TRY target out of bounds".to_string());
                    continue;
                }
                try_frames.push(TryFrame {
                    catch_ip,
                    finally_ip,
                    caught: false,
                    in_finally: false,
                    end_ip: 0,
                })?;
                ip += 3;
                continue;
            }
            ENDTRY => {
                // ENDTRY offset_i8: end of try/catch block
                if ip + 2 > script.len() {
                    pending_error = Some("truncated ENDTRY operand".to_string());
                    continue;
                }
                let offset = script[ip + 1] as i8;
                let target_ip = if offset != 0 {
                    (ip as isize + offset as isize) as usize
                } else {
                    ip + 2
                };
                if let Some(frame) = try_frames.last_mut() {
                    if frame.finally_ip != 0 && !frame.in_finally {
                        // Save continuation IP for ENDFINALLY to use
                        frame.end_ip = target_ip;
                        frame.in_finally = true;
                        ip = frame.finally_ip;
                        continue;
                    }
                }
                // No finally or already in finally — pop frame and jump
                try_frames.pop();
                ip = target_ip;
                continue;
            }
            TRY_L => {
                // TRY_L: long-form TRY with i32 catch_offset, i32 finally_offset (9 bytes)
                if ip + 9 > script.len() {
                    pending_error = Some("truncated TRY_L operand".to_string());
                    continue;
                }
                let catch_offset = i32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]);
                let finally_offset = i32::from_le_bytes([
                    script[ip + 5],
                    script[ip + 6],
                    script[ip + 7],
                    script[ip + 8],
                ]);
                let catch_ip = if catch_offset != 0 {
                    (ip as isize + catch_offset as isize) as usize
                } else {
                    0
                };
                let finally_ip = if finally_offset != 0 {
                    (ip as isize + finally_offset as isize) as usize
                } else {
                    0
                };
                if catch_ip > script.len() || finally_ip > script.len() {
                    pending_error = Some("TRY_L target out of bounds".to_string());
                    continue;
                }
                try_frames.push(TryFrame {
                    catch_ip,
                    finally_ip,
                    caught: false,
                    in_finally: false,
                    end_ip: 0,
                })?;
                ip += 9;
                continue;
            }
            ENDTRY_L => {
                // ENDTRY_L offset_i32: long-form ENDTRY (5 bytes)
                if ip + 5 > script.len() {
                    pending_error = Some("truncated ENDTRY_L operand".to_string());
                    continue;
                }
                let offset = i32::from_le_bytes([
                    script[ip + 1],
                    script[ip + 2],
                    script[ip + 3],
                    script[ip + 4],
                ]);
                let target_ip = if offset != 0 {
                    (ip as isize + offset as isize) as usize
                } else {
                    ip + 5
                };
                if let Some(frame) = try_frames.last_mut() {
                    if frame.finally_ip != 0 && !frame.in_finally {
                        frame.end_ip = target_ip;
                        frame.in_finally = true;
                        ip = frame.finally_ip;
                        continue;
                    }
                }
                try_frames.pop();
                ip = target_ip;
                continue;
            }
            ENDFINALLY => {
                // ENDFINALLY must be reached via the finally path
                let in_finally = try_frames.last_mut().is_some_and(|f| f.in_finally);
                if in_finally {
                    let frame = try_frames.pop().unwrap();
                    if pending_error.is_some() {
                        // Re-throw: pending_error will be processed at top of loop
                        continue;
                    }
                    // Normal completion after finally — jump to saved end_ip
                    if frame.end_ip != 0 {
                        ip = frame.end_ip;
                        continue;
                    }
                } else {
                    // ENDFINALLY without being in finally state = FAULT
                    return Err("ENDFINALLY without matching finally context".to_string());
                }
            }
            other => {
                return Err(format!("unsupported opcode 0x{other:02x}"));
            }
        }
        ip += 1;
    }

    Ok(ExecutionResult {
        fee_consumed_pico: 0,
        state: VmState::Halt,
        stack: to_abi_stack(&stack),
        fault_message: None,
    })
}

struct NoSyscalls;

impl SyscallProvider for NoSyscalls {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<AbiStackValue>,
    ) -> Result<(), String> {
        Err(format!("unsupported syscall 0x{api:08x}"))
    }
}

#[cfg(test)]
mod try_catch_tests {
    use super::*;
    use neo_riscv_abi::StackValue;

    struct ErrorSyscall;
    impl SyscallProvider for ErrorSyscall {
        fn on_instruction(&mut self, _opcode: u8) -> Result<(), String> {
            Ok(())
        }
        fn syscall(
            &mut self,
            _api: u32,
            _ip: usize,
            _stack: &mut Vec<StackValue>,
        ) -> Result<(), String> {
            Err("error".to_string())
        }
    }

    #[test]
    fn test_try_catch_syscall_exception() {
        let script: Vec<u8> = vec![
            0x3b, 0x0a, 0x00, // TRY catch_offset=10, finally_offset=0
            0x41, 0xde, 0xad, 0xde, 0xad, // SYSCALL
            0x3d, 0x05, // ENDTRY offset=5
            0x11, // PUSH1
            0x3d, 0x02, // ENDTRY offset=2
            0x12, // PUSH2
        ];
        let mut provider = ErrorSyscall;
        let result =
            interpret_with_stack_and_syscalls_at(&script, Vec::new(), 0, &mut provider).unwrap();
        assert_eq!(result.state, VmState::Halt);
        assert_eq!(result.stack.len(), 3);
    }
}
