//! Contract entry point harness for C#-compiled PolkaVM smart contracts.
//!
//! Provides the boilerplate to decode the incoming ABI stack, extract the
//! method name, build a `Context`, and encode the execution result. Generated
//! contracts use these helpers in their PolkaVM `execute` export.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
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
    postcard::to_allocvec(&wrapped).unwrap_or_default()
}

// ---------------------------------------------------------------
// Guest-side diagnostic tracing
// ---------------------------------------------------------------

const DEBUG_BUF_SIZE: usize = 4096;
static mut DEBUG_BUF: [u8; DEBUG_BUF_SIZE] = [0u8; DEBUG_BUF_SIZE];
static mut DEBUG_LEN: usize = 0;

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

    // GetContext returns an opaque context handle — we use Integer(0) as a
    // sentinel.  Avoids a host round-trip and works around a PolkaVM issue
    // where heap allocations preceding an ecalli can corrupt nearby guest
    // memory (bump-allocator + ecalli interaction).
    if hash == 0xce67f69b {
        ctx.push(StackValue::Integer(0));
        return;
    }

    // RAW entry record — write before anything else to confirm entry
    debug_record(0xFF, hash, ctx.stack.len() as u32, 0, 0, 0, None);

    // Determine how many arguments this syscall needs
    let arg_count = syscall_arg_count(hash);
    let stack_len = ctx.stack.len();
    let actual_count = if arg_count == usize::MAX {
        ctx.stack.len()
    } else {
        arg_count.min(ctx.stack.len())
    };

    debug_record(0, hash, stack_len as u32, actual_count as u32, 0, 0, None);

    // Pop arguments from the evaluation stack (top-of-stack first).
    // abi_args[0] = top-of-stack (last-pushed arg), which is the first
    // parameter the host callback expects (context for Storage.Put, etc.).
    let mut abi_args: Vec<AbiStackValue> = Vec::with_capacity(actual_count);
    for _ in 0..actual_count {
        abi_args.push(ctx.pop().to_abi());
    }

    // Encode arguments using fast_codec (the host decodes with fast_codec)
    let encoded = fast_codec::encode_stack(&abi_args);

    debug_record(
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

    debug_record(
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
        debug_record(3, hash, 0, 0, 0, 0, Some("result_len=0, no data"));
        return;
    }

    let result_data = &result_buf[..result_len as usize];

    // Log first few bytes of result for debugging
    let b0 = if result_len > 0 { result_data[0] } else { 0xFF };
    let b1 = if result_len > 1 { result_data[1] } else { 0xFF };
    debug_record(7, hash, result_len as u32, b0 as u32, b1 as u32, 0, None);

    // Decode the callback_codec result (the host encodes with callback_codec)
    match callback_codec::decode_stack_result(result_data) {
        Ok(Ok(items)) => {
            let count = items.len();
            debug_record(8, hash, count as u32, 0, 0, 0, None);
            for item in items {
                ctx.push(StackValue::from_abi(&item));
            }
            debug_record(4, hash, count as u32, 0, 0, result_len, None);
        }
        Ok(Err(e)) => {
            let msg = alloc::format!("host error: {}", e);
            debug_record(5, hash, 0, 0, 0, result_len, Some(&msg));
            ctx.fault(&format!("syscall 0x{:08x} error: {}", hash, e));
        }
        Err(e) => {
            let msg = alloc::format!("decode error: {:?}", e);
            debug_record(6, hash, 0, 0, 0, result_len, Some(&msg));
            ctx.fault(&format!("syscall 0x{:08x} decode error: {}", hash, e));
        }
    }
    debug_record(9, hash, ctx.stack.len() as u32, 0, 0, 0, None);
}
