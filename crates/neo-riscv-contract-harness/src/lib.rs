//! Contract entry point harness for C#-compiled PolkaVM smart contracts.
//!
//! Provides the boilerplate to decode the incoming ABI stack, extract the
//! method name, build a `Context`, and encode the execution result. Generated
//! contracts use these helpers in their PolkaVM `execute` export.

#![no_std]

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;
use neo_riscv_abi::callback_codec;
use neo_riscv_abi::fast_codec;
use neo_riscv_abi::StackValue as AbiStackValue;
use neo_riscv_rt::Context;

// === PolkaVM host import ===
// Declared here (not in generated contracts) so bridge_syscall can call it
// directly. PolkaVM import thunks are not regular function pointers — they
// depend on internal PolkaVM state that is only valid within the declaring
// crate, so storing them in a static and re-calling later fails.
#[polkavm_derive::polkavm_import]
extern "C" {
    fn host_call(
        api: u32,
        ip: u32,
        stack_ptr: u32,
        stack_len: u32,
        result_ptr: u32,
        result_cap: u32,
    ) -> u32;
}

/// Result of decoding the entry stack.
pub struct EntryResult {
    /// The method name extracted from the first stack item (ByteString).
    pub method_name: Vec<u8>,
    /// The execution context populated with the remaining arguments.
    pub ctx: Context,
}

/// Decode the initial stack passed by the host and extract the method name.
///
/// The host sends a `fast_codec`-encoded stack where the first item is a
/// `ByteString` containing the method name. The remaining items become the
/// evaluation stack for the contract method.
///
/// On decode failure, returns an `EntryResult` with an empty method name and
/// a faulted context.
pub fn decode_entry(stack_data: &[u8]) -> EntryResult {
    let abi_stack = match fast_codec::decode_stack(stack_data) {
        Ok(s) => s,
        Err(_) => {
            let mut ctx = Context::from_abi_stack(Vec::new());
            ctx.fault("failed to decode stack");
            return EntryResult {
                method_name: Vec::new(),
                ctx,
            };
        }
    };

    let method_name = match abi_stack.first() {
        Some(AbiStackValue::ByteString(b)) => b.clone(),
        _ => Vec::new(),
    };

    let args: Vec<AbiStackValue> = if abi_stack.len() > 1 {
        abi_stack[1..].to_vec()
    } else {
        Vec::new()
    };

    let ctx = Context::from_abi_stack(args);
    EntryResult { method_name, ctx }
}

/// Decode a fast_codec-encoded ABI stack directly into an execution context.
///
/// Unlike `decode_entry`, this helper expects no leading method-name item.
/// It is used by the native-contract fast entrypoint where the host passes
/// the selected method separately as an integer identifier.
pub fn decode_context(stack_data: &[u8]) -> Context {
    match fast_codec::decode_stack(stack_data) {
        Ok(stack) => Context::from_abi_stack(stack),
        Err(_) => {
            let mut ctx = Context::from_abi_stack(Vec::new());
            ctx.fault("failed to decode stack");
            ctx
        }
    }
}

/// Encode the execution context as a postcard-serialized `Result<ExecutionResult, String>`.
///
/// The host expects `postcard::from_bytes::<Result<ExecutionResult, String>>()`,
/// so we must wrap the `ExecutionResult` in `Ok(...)` before serializing.
///
/// Returns the serialized bytes suitable for writing into the result buffer
/// that the host reads via `get_result_ptr` / `get_result_len`.
pub fn encode_result(ctx: Context) -> Vec<u8> {
    let result = ctx.to_execution_result(0);
    let wrapped: Result<neo_riscv_abi::ExecutionResult, alloc::string::String> = Ok(result);
    neo_riscv_abi::result_codec::encode_execution_result(&wrapped)
}

// ---------------------------------------------------------------
// Guest-side diagnostic tracing
// ---------------------------------------------------------------

const DEBUG_BUF_SIZE: usize = 4096;
static mut DEBUG_BUF: [u8; DEBUG_BUF_SIZE] = [0u8; DEBUG_BUF_SIZE];
static mut DEBUG_LEN: usize = 0;
const ENABLE_SYSCALL_DEBUG: bool = false;

/// Write a diagnostic record into the debug buffer.
///
/// Format: [step:u8, api:u32 LE, stack_len:u32 LE, arg_count:u32 LE,
///          encoded_len:u32 LE, result_len:u32 LE, fault_msg_len:u16 LE, fault_msg...]
fn debug_record(
    step: u8,
    api: u32,
    stack_len: u32,
    arg_count: u32,
    encoded_len: u32,
    result_len: u32,
    fault: Option<&str>,
) {
    unsafe {
        let needed = 23 + fault.map_or(0, |s| 2 + s.len());
        if DEBUG_LEN + needed > DEBUG_BUF_SIZE {
            return;
        }
        let base = DEBUG_BUF.as_mut_ptr().add(DEBUG_LEN);
        base.write(step);
        base.add(1).cast::<u32>().write(api);
        base.add(5).cast::<u32>().write(stack_len);
        base.add(9).cast::<u32>().write(arg_count);
        base.add(13).cast::<u32>().write(encoded_len);
        base.add(17).cast::<u32>().write(result_len);
        if let Some(msg) = fault {
            let msg_bytes = msg.as_bytes();
            base.add(21).cast::<u16>().write(msg_bytes.len() as u16);
            core::ptr::copy_nonoverlapping(msg_bytes.as_ptr(), base.add(23), msg_bytes.len());
            DEBUG_LEN += 23 + msg_bytes.len();
        } else {
            base.add(21).cast::<u16>().write(0);
            DEBUG_LEN += 23;
        }
    }
}

#[inline]
fn maybe_debug_record(
    step: u8,
    api: u32,
    stack_len: u32,
    arg_count: u32,
    encoded_len: u32,
    result_len: u32,
    fault: Option<&str>,
) {
    if ENABLE_SYSCALL_DEBUG {
        debug_record(
            step,
            api,
            stack_len,
            arg_count,
            encoded_len,
            result_len,
            fault,
        );
    }
}

/// Export: pointer to the debug buffer.
pub fn get_debug_ptr() -> u32 {
    unsafe { DEBUG_BUF.as_ptr() as u32 }
}

/// Export: number of bytes written to the debug buffer.
pub fn get_debug_len() -> u32 {
    unsafe { DEBUG_LEN as u32 }
}

/// Reset the debug buffer.
pub fn reset_debug() {
    unsafe {
        DEBUG_LEN = 0;
    }
}

// ---------------------------------------------------------------
// Syscall bridge
// ---------------------------------------------------------------

/// Execute a syscall by marshaling arguments through the host boundary.
///
/// 1. Pops the required number of arguments from the context's evaluation stack
/// 2. Encodes them with `fast_codec` (matches what the host expects)
/// 3. Calls through to the PolkaVM `host_call` import
/// 4. Decodes the `callback_codec` result and pushes items back onto the stack
pub fn bridge_syscall(ctx: &mut Context, hash: u32) {
    use alloc::format;
    use alloc::vec;
    use neo_riscv_abi::{callback_codec, syscall_arg_count};
    use neo_riscv_rt::stack_value::StackValue;

    // Runtime.CheckWitness often appears in auth guards directly before ASSERT.
    // Avoid heap allocations on this path to reduce the chance of guest-memory
    // corruption around ecalli in the PolkaVM bump-allocator environment.
    if hash == 0x8cec27f8 {
        if try_check_witness_fast_path(ctx, hash) {
            return;
        }
    }

    // RAW entry record — write before anything else to confirm entry
    maybe_debug_record(0xFF, hash, ctx.stack.len() as u32, 0, 0, 0, None);

    // Determine how many arguments this syscall needs
    let arg_count = syscall_arg_count(hash);
    let stack_len = ctx.stack.len();
    let actual_count = if arg_count == usize::MAX {
        ctx.stack.len()
    } else {
        arg_count.min(ctx.stack.len())
    };

    maybe_debug_record(0, hash, stack_len as u32, actual_count as u32, 0, 0, None);

    // Pop arguments from the evaluation stack (top-of-stack first).
    // abi_args[0] = top-of-stack (last-pushed arg), which is the first
    // parameter the host callback expects (context for Storage.Put, etc.).
    let mut abi_args: Vec<AbiStackValue> = Vec::with_capacity(actual_count);
    for _ in 0..actual_count {
        abi_args.push(ctx.pop().to_abi());
    }

    // Encode arguments using fast_codec (the host decodes with fast_codec)
    let encoded = fast_codec::encode_stack(&abi_args);

    maybe_debug_record(
        1,
        hash,
        stack_len as u32,
        actual_count as u32,
        encoded.len() as u32,
        0,
        None,
    );

    // Prepare a result buffer
    let mut result_buf = vec![0u8; 4096];

    // Call through to the host directly via the PolkaVM import thunk
    let result_len = unsafe {
        host_call(
            hash,
            0,
            encoded.as_ptr() as u32,
            encoded.len() as u32,
            result_buf.as_mut_ptr() as u32,
            result_buf.len() as u32,
        )
    };

    maybe_debug_record(
        2,
        hash,
        stack_len as u32,
        actual_count as u32,
        encoded.len() as u32,
        result_len,
        None,
    );

    if result_len == 0 {
        // No result data — syscall returned nothing (valid for void syscalls)
        maybe_debug_record(3, hash, 0, 0, 0, 0, Some("result_len=0, no data"));
        return;
    }

    let result_data = &result_buf[..result_len as usize];

    if try_decode_small_callback_result(ctx, result_data) {
        let count = ctx.stack.len();
        maybe_debug_record(4, hash, count as u32, 0, 0, result_len, None);
        maybe_debug_record(9, hash, count as u32, 0, 0, 0, None);
        return;
    }

    // Log first few bytes of result for debugging
    let b0 = if result_len > 0 { result_data[0] } else { 0xFF };
    let b1 = if result_len > 1 { result_data[1] } else { 0xFF };
    maybe_debug_record(7, hash, result_len as u32, b0 as u32, b1 as u32, 0, None);

    // Decode the callback_codec result (the host encodes with callback_codec)
    match callback_codec::decode_stack_result(result_data) {
        Ok(Ok(items)) => {
            let count = items.len();
            maybe_debug_record(8, hash, count as u32, 0, 0, 0, None);
            for item in items {
                ctx.push(StackValue::from_abi(&item));
            }
            maybe_debug_record(4, hash, count as u32, 0, 0, result_len, None);
        }
        Ok(Err(e)) => {
            let msg = alloc::format!("host error: {}", e);
            maybe_debug_record(5, hash, 0, 0, 0, result_len, Some(&msg));
            ctx.fault(&format!("syscall 0x{:08x} error: {}", hash, e));
        }
        Err(e) => {
            let msg = alloc::format!("decode error: {:?}", e);
            maybe_debug_record(6, hash, 0, 0, 0, result_len, Some(&msg));
            ctx.fault(&format!("syscall 0x{:08x} decode error: {}", hash, e));
        }
    }
    maybe_debug_record(9, hash, ctx.stack.len() as u32, 0, 0, 0, None);
}

fn try_check_witness_fast_path(ctx: &mut Context, hash: u32) -> bool {
    use neo_riscv_rt::stack_value::StackValue;

    const MAX_WITNESS_BYTES: usize = 64;
    const ENCODED_CAP: usize = 4 + 1 + 4 + MAX_WITNESS_BYTES;
    const RESULT_CAP: usize = 32;

    let (tag, witness) = match ctx.stack.last() {
        Some(StackValue::ByteString(bytes)) if bytes.len() <= MAX_WITNESS_BYTES => (0x03u8, bytes),
        Some(StackValue::Buffer(bytes)) if bytes.len() <= MAX_WITNESS_BYTES => (0x0Cu8, bytes),
        _ => return false,
    };

    let mut encoded = [0u8; ENCODED_CAP];
    encoded[..4].copy_from_slice(&1u32.to_le_bytes());
    encoded[4] = tag;
    encoded[5..9].copy_from_slice(&(witness.len() as u32).to_le_bytes());
    encoded[9..9 + witness.len()].copy_from_slice(witness);
    let encoded_len = 9 + witness.len();

    let _ = ctx.pop();

    let mut result_buf = [0u8; RESULT_CAP];
    let result_len = unsafe {
        host_call(
            hash,
            0,
            encoded.as_ptr() as u32,
            encoded_len as u32,
            result_buf.as_mut_ptr() as u32,
            RESULT_CAP as u32,
        )
    };

    if result_len == 0 {
        return true;
    }

    let result_data = &result_buf[..result_len as usize];
    if try_decode_small_callback_result(ctx, result_data) {
        return true;
    }

    match callback_codec::decode_stack_result(result_data) {
        Ok(Ok(items)) => {
            for item in items {
                ctx.push(StackValue::from_abi(&item));
            }
        }
        Ok(Err(e)) => ctx.fault(&format!("syscall 0x{:08x} error: {}", hash, e)),
        Err(e) => ctx.fault(&format!("syscall 0x{:08x} decode error: {}", hash, e)),
    }

    true
}

fn try_decode_small_callback_result(ctx: &mut Context, result_data: &[u8]) -> bool {
    use neo_riscv_rt::stack_value::StackValue;

    match result_data {
        [2] => true,
        [5] => {
            ctx.push(StackValue::Null);
            true
        }
        [3, a, b, c, d, e, f, g, h] => {
            ctx.push(StackValue::Integer(i64::from_le_bytes([
                *a, *b, *c, *d, *e, *f, *g, *h,
            ])));
            true
        }
        [4, flag] => {
            ctx.push(StackValue::Boolean(*flag != 0));
            true
        }
        [6, l0, l1, l2, l3, rest @ ..] => {
            let len = u32::from_le_bytes([*l0, *l1, *l2, *l3]) as usize;
            if rest.len() != len {
                return false;
            }
            ctx.push(StackValue::ByteString(rest.to_vec()));
            true
        }
        [7, l0, l1, l2, l3, rest @ ..] => {
            let len = u32::from_le_bytes([*l0, *l1, *l2, *l3]) as usize;
            if rest.len() != len {
                return false;
            }
            ctx.push(StackValue::BigInteger(rest.to_vec()));
            true
        }
        [8, a, b, c, d, e, f, g, h] => {
            ctx.push(StackValue::Interop(u64::from_le_bytes([
                *a, *b, *c, *d, *e, *f, *g, *h,
            ])));
            true
        }
        [9, a, b, c, d, e, f, g, h] => {
            ctx.push(StackValue::Iterator(u64::from_le_bytes([
                *a, *b, *c, *d, *e, *f, *g, *h,
            ])));
            true
        }
        [10, a, b, c, d, e, f, g, h] => {
            ctx.push(StackValue::Pointer(i64::from_le_bytes([
                *a, *b, *c, *d, *e, *f, *g, *h,
            ])));
            true
        }
        [11, l0, l1, l2, l3, rest @ ..] => {
            let len = u32::from_le_bytes([*l0, *l1, *l2, *l3]) as usize;
            if rest.len() != len {
                return false;
            }
            ctx.push(StackValue::Buffer(rest.to_vec()));
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::try_decode_small_callback_result;
    use alloc::vec;
    use neo_riscv_abi::VmState;
    use neo_riscv_rt::Context;

    #[test]
    fn small_boolean_callback_result_survives_assert_top() {
        let mut ctx = Context::from_abi_stack(vec![]);

        assert!(try_decode_small_callback_result(&mut ctx, &[4, 1]));
        ctx.assert_top();

        assert_eq!(ctx.state, VmState::Halt);
        assert!(ctx.stack.is_empty());
        assert!(ctx.fault_message.is_none());
    }
}
