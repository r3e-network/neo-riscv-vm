#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

extern crate alloc;

#[cfg(target_arch = "riscv32")]
mod mem_intrinsics;

mod aligned_arena;
#[cfg(target_arch = "riscv32")]
mod buf_writer;
mod bump_state;
mod polkavm_syscall_provider;
mod raw_buffer;
mod resettable_bump_allocator;
mod runtime_state;
mod runtime_state_cell;

use alloc::vec::Vec;

// Increase PolkaVM call stack from the default size to handle deep codec
// decoding (e.g., nested arrays in Contract.Call results at mainnet block 78538).
#[cfg(all(
    any(target_arch = "riscv32", target_arch = "riscv64"),
    target_feature = "e"
))]
polkavm_derive::min_stack_size!(1048576);
use aligned_arena::AlignedArena;
#[cfg(target_arch = "riscv32")]
use buf_writer::BufWriter;
use neo_riscv_abi::{fast_codec, ExecutionResult, StackValue};
use polkavm_syscall_provider::PolkaVmSyscallProvider;
use resettable_bump_allocator::ResettableBumpAllocator;
use runtime_state::RuntimeState;
use runtime_state_cell::RuntimeStateCell;

// Keep enough heap headroom for 1MB MaxItemSize allocations alongside
// interpreter state and callback decoding buffers. Mainnet NeoVM compatibility
// needs this above 64 MiB for large historical stack-heavy calls such as
// GhostMarket.NFT.fixRoyalties at block 470449 and GhostMarket:_initialize at
// block 2368696, plus allocation-heavy Contract.Call execution at block
// 2655903.
const SCRATCH_BUF_SIZE: usize = 2 * 1024 * 1024;
const ALLOC_ARENA_SIZE: usize = 128 * 1024 * 1024;
const ARENA_SIZE: usize = ALLOC_ARENA_SIZE + SCRATCH_BUF_SIZE * 2;
const ALLOC_PRETOUCH_SIZE: usize = 1024 * 1024;
const PANIC_BUF_SIZE: usize = 256;
const TRACE_HEAD_SIZE: usize = 32;

static ARENA: AlignedArena = AlignedArena::new();

const ALLOC_BASE_OFFSET: usize = 0;
const REQ_BUF_OFFSET: usize = ALLOC_ARENA_SIZE;
const RES_BUF_OFFSET: usize = REQ_BUF_OFFSET + SCRATCH_BUF_SIZE;

const _: () = assert!(ALLOC_BASE_OFFSET + ALLOC_ARENA_SIZE <= ARENA_SIZE);
const _: () = assert!(ALLOC_PRETOUCH_SIZE <= ALLOC_ARENA_SIZE);
const _: () = assert!(RES_BUF_OFFSET + SCRATCH_BUF_SIZE <= ARENA_SIZE);

#[cfg_attr(target_arch = "riscv32", link_section = ".data.neo_riscv_state")]
static RUNTIME_STATE: RuntimeStateCell = RuntimeStateCell::new();

unsafe fn runtime_state() -> &'static mut RuntimeState {
    RUNTIME_STATE.get_mut()
}

unsafe fn arena_ptr(offset: usize) -> *mut u8 {
    ARENA.as_mut_ptr().add(offset)
}

unsafe fn arena_slice(offset: usize, len: usize) -> &'static mut [u8] {
    core::slice::from_raw_parts_mut(arena_ptr(offset), len)
}

unsafe fn pretouch_alloc_arena() {
    if ALLOC_PRETOUCH_SIZE > 0 {
        core::ptr::write_volatile(arena_ptr(ALLOC_BASE_OFFSET + ALLOC_PRETOUCH_SIZE - 1), 0);
    }
}

#[cfg_attr(target_arch = "riscv32", global_allocator)]
static ALLOCATOR: ResettableBumpAllocator = ResettableBumpAllocator::new();

/// Capture panic message into PANIC_BUF for host-side diagnostics.
/// No heap allocation: writes directly into the static buffer.
#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        let buf = &mut runtime_state().panic_buf[..];
        let mut w = BufWriter::new(buf);
        let _ = core::fmt::write(&mut w, format_args!("{info}"));
        runtime_state().panic_len = w.len() as u32;
    }
    unsafe {
        core::arch::asm!("unimp");
    }
    loop {}
}

#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub extern "C" fn _start() {}

#[cfg(target_arch = "riscv32")]
#[no_mangle]
pub extern "C" fn main() {}

#[cfg(not(target_arch = "riscv32"))]
fn main() {}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn alloc(size: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    core::mem::forget(buf);
    ptr
}

#[cfg(not(test))]
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
    fn host_on_instruction(opcode: u32) -> u32;
}

#[cfg(test)]
#[no_mangle]
unsafe extern "C" fn host_call(
    _api: u32,
    _ip: u32,
    _stack_ptr: u32,
    _stack_len: u32,
    _result_ptr: u32,
    _result_cap: u32,
) -> u32 {
    0
}

#[cfg(test)]
#[no_mangle]
unsafe extern "C" fn host_on_instruction(_opcode: u32) -> u32 {
    1
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_result_ptr() -> *const u8 {
    unsafe { runtime_state().result_ptr as *const u8 }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_result_len() -> u32 {
    unsafe { runtime_state().result_len }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_panic_ptr() -> *const u8 {
    unsafe { runtime_state().panic_buf.as_ptr() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_panic_len() -> u32 {
    unsafe { runtime_state().panic_len }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_res_len() -> u32 {
    unsafe { runtime_state().trace_res_len }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_res_head_ptr() -> *const u8 {
    unsafe { runtime_state().trace_res_head.as_ptr() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_stage() -> u32 {
    unsafe { runtime_state().trace_syscall_stage }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_api() -> u32 {
    unsafe { runtime_state().trace_syscall_api }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_ip() -> u32 {
    unsafe { runtime_state().trace_syscall_ip }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_last_interpreter_ip() -> u32 {
    neo_riscv_guest::last_interpreter_ip()
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_last_result_stage() -> u32 {
    neo_riscv_guest::last_result_stage()
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_last_result_stack_len() -> u32 {
    neo_riscv_guest::last_result_stack_len()
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_last_result_limit() -> u32 {
    neo_riscv_guest::last_result_limit()
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_req_len() -> u32 {
    unsafe { runtime_state().trace_req_len }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_stack_items() -> u32 {
    unsafe { runtime_state().trace_stack_items }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_allocator_peak() -> u32 {
    unsafe { ALLOCATOR.peak_bytes() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_allocator_fail_count() -> u32 {
    unsafe { ALLOCATOR.fail_count() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_allocator_fail_size() -> u32 {
    unsafe { ALLOCATOR.fail_size() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_allocator_fail_align() -> u32 {
    unsafe { ALLOCATOR.fail_align() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn execute(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
) {
    let result = execute_inner(
        script_ptr,
        script_len,
        stack_ptr,
        stack_len,
        initial_ip,
        None,
        u32::MAX,
    );
    store_result(result, u32::MAX);
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn execute_with_result_limit(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
    result_limit: u32,
) {
    let result = execute_inner(
        script_ptr,
        script_len,
        stack_ptr,
        stack_len,
        initial_ip,
        None,
        result_limit,
    );
    store_result(result, result_limit);
}

static mut NEXT_RESULT_LIMIT: u32 = u32::MAX;

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn set_result_limit(result_limit: u32) {
    unsafe {
        NEXT_RESULT_LIMIT = result_limit;
    }
}

fn take_result_limit() -> u32 {
    unsafe {
        let result_limit = NEXT_RESULT_LIMIT;
        NEXT_RESULT_LIMIT = u32::MAX;
        result_limit
    }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn execute_with_initializer(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
    initializer_ip: u32,
) {
    let result_limit = take_result_limit();
    let result = execute_inner(
        script_ptr,
        script_len,
        stack_ptr,
        stack_len,
        initial_ip,
        Some(initializer_ip),
        result_limit,
    );
    store_result(result, result_limit);
}

fn store_result(mut result: Result<ExecutionResult, alloc::string::String>, result_limit: u32) {
    if let Ok(ref mut execution_result) = result {
        if matches!(execution_result.state, neo_riscv_abi::VmState::Halt)
            && result_limit != u32::MAX
        {
            let keep = result_limit as usize;
            if keep == 0 {
                execution_result.stack.clear();
            } else if execution_result.stack.len() > keep {
                let start = execution_result.stack.len() - keep;
                execution_result.stack.drain(0..start);
            }
        }
    }

    unsafe {
        let bytes = neo_riscv_abi::result_codec::encode_execution_result(&result);
        let state = runtime_state();
        state.result_ptr = if bytes.is_empty() {
            0
        } else {
            bytes.as_ptr() as u32
        };
        state.result_len = bytes.len().min(u32::MAX as usize) as u32;
        core::mem::forget(bytes);
    }
}

fn execute_inner(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
    initializer_ip: Option<u32>,
    result_limit: u32,
) -> Result<ExecutionResult, alloc::string::String> {
    unsafe {
        ALLOCATOR.reset();
        pretouch_alloc_arena();
        let state = runtime_state();
        state.result_ptr = 0;
        state.result_len = 0;
        state.panic_len = 0;
        state.trace_syscall_stage = 0;
        state.trace_syscall_api = 0;
        state.trace_syscall_ip = 0;
        state.trace_req_len = 0;
        state.trace_stack_items = 0;
        state.trace_res_len = 0;
        state.trace_res_head.fill(0);
    }
    let script =
        unsafe { core::slice::from_raw_parts(script_ptr as *const u8, script_len as usize) };

    let initial_stack: Vec<StackValue> = if stack_len > 0 {
        let stack_bytes =
            unsafe { core::slice::from_raw_parts(stack_ptr as *const u8, stack_len as usize) };
        fast_codec::decode_stack(stack_bytes)
            .map_err(|e| alloc::format!("failed to deserialize initial stack: {e}"))?
    } else {
        Vec::new()
    };

    let mut provider = PolkaVmSyscallProvider;
    if let Some(initializer_ip) = initializer_ip {
        if result_limit == u32::MAX {
            neo_riscv_guest::interpret_with_stack_and_syscalls_at_with_initializer(
                script,
                initial_stack,
                initial_ip as usize,
                initializer_ip as usize,
                &mut provider,
            )
        } else {
            neo_riscv_guest::interpret_with_stack_and_syscalls_at_with_initializer_and_result_limit(
                script,
                initial_stack,
                initial_ip as usize,
                initializer_ip as usize,
                result_limit as usize,
                &mut provider,
            )
        }
    } else if result_limit == u32::MAX {
        neo_riscv_guest::interpret_with_stack_and_syscalls_at(
            script,
            initial_stack,
            initial_ip as usize,
            &mut provider,
        )
    } else {
        neo_riscv_guest::interpret_with_stack_and_syscalls_at_with_result_limit(
            script,
            initial_stack,
            initial_ip as usize,
            result_limit as usize,
            &mut provider,
        )
    }
}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use super::ALLOC_ARENA_SIZE;

    #[test]
    fn mainnet_470449_requires_heap_headroom_above_16_mib() {
        assert!(
            black_box(ALLOC_ARENA_SIZE) >= 32 * 1024 * 1024,
            "mainnet block 470449 GhostMarket.NFT.fixRoyalties exhausted a 16 MiB guest arena"
        );
    }

    #[test]
    fn mainnet_2368696_requires_heap_headroom_above_32_mib() {
        assert!(
            black_box(ALLOC_ARENA_SIZE) >= 64 * 1024 * 1024,
            "mainnet block 2368696 GhostMarket:_initialize exhausted a 32 MiB guest arena"
        );
    }

    #[test]
    fn mainnet_2655903_requires_heap_headroom_above_64_mib() {
        assert!(
            black_box(ALLOC_ARENA_SIZE) >= 128 * 1024 * 1024,
            "mainnet block 2655903 exhausted a 64 MiB guest arena during Contract.Call execution"
        );
    }
}
