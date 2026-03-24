#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

extern crate alloc;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use neo_riscv_abi::{callback_codec, ExecutionResult, StackValue};
use neo_riscv_guest::SyscallProvider;

const ARENA_SIZE: usize = 256 * 1024 * 1024;
const SCRATCH_BUF_SIZE: usize = 1024 * 1024;
const PANIC_BUF_SIZE: usize = 256;
const TRACE_HEAD_SIZE: usize = 32;

struct RawBuffer<const N: usize>(UnsafeCell<[u8; N]>);

unsafe impl<const N: usize> Sync for RawBuffer<N> {}

impl<const N: usize> RawBuffer<N> {
    const fn new() -> Self {
        Self(UnsafeCell::new([0; N]))
    }

    const fn len(&self) -> usize {
        N
    }

    unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.0.get().cast::<u8>()
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn as_mut_slice(&self, len: usize) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}

struct StaticVec(UnsafeCell<Vec<u8>>);

unsafe impl Sync for StaticVec {}

impl StaticVec {
    const fn new() -> Self {
        Self(UnsafeCell::new(Vec::new()))
    }

    unsafe fn replace(&self, value: Vec<u8>) {
        *self.0.get() = value;
    }

    unsafe fn as_ptr(&self) -> *const u8 {
        (*self.0.get()).as_ptr()
    }

    unsafe fn len(&self) -> u32 {
        (*self.0.get()).len() as u32
    }
}

#[repr(align(64))]
struct AlignedArena(RawBuffer<ARENA_SIZE>);

unsafe impl Sync for AlignedArena {}

impl AlignedArena {
    const fn new() -> Self {
        Self(RawBuffer::new())
    }

    unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

static ARENA: AlignedArena = AlignedArena::new();
static REQ_BUF: RawBuffer<SCRATCH_BUF_SIZE> = RawBuffer::new();
/// Fixed 1MB buffer for syscall responses from host to guest.
/// Uses a static buffer instead of dynamic Vec to avoid talc allocator corruption
/// under PolkaVM's RISC-V memory model. Guest copies the response into this buffer
/// via host_call(), then decodes it back to Vec<StackValue>.
static RES_BUF: RawBuffer<SCRATCH_BUF_SIZE> = RawBuffer::new();

#[derive(Default)]
struct BumpState {
    offset: usize,
    peak: usize,
    fail_count: u32,
    fail_size: usize,
    fail_align: usize,
}

struct ResettableBumpAllocator(UnsafeCell<BumpState>);

unsafe impl Sync for ResettableBumpAllocator {}

impl ResettableBumpAllocator {
    const fn new() -> Self {
        Self(UnsafeCell::new(BumpState {
            offset: 0,
            peak: 0,
            fail_count: 0,
            fail_size: 0,
            fail_align: 0,
        }))
    }

    unsafe fn reset(&self) {
        let state = &mut *self.0.get();
        state.offset = 0;
        state.peak = 0;
        state.fail_count = 0;
        state.fail_size = 0;
        state.fail_align = 0;
    }

    unsafe fn alloc_from_arena(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return NonNull::<u8>::dangling().as_ptr();
        }

        let state = &mut *self.0.get();
        let base = ARENA.as_mut_ptr() as usize;
        let current = base + state.offset;
        let align_mask = layout.align() - 1;
        let aligned = (current + align_mask) & !align_mask;
        let end = aligned.saturating_add(layout.size());
        if end > base + ARENA_SIZE {
            state.fail_count = state.fail_count.saturating_add(1);
            state.fail_size = layout.size();
            state.fail_align = layout.align();
            return core::ptr::null_mut();
        }

        state.offset = end - base;
        state.peak = core::cmp::max(state.peak, state.offset);
        aligned as *mut u8
    }

    unsafe fn peak_bytes(&self) -> u32 {
        (*self.0.get()).peak.min(u32::MAX as usize) as u32
    }

    unsafe fn fail_count(&self) -> u32 {
        (*self.0.get()).fail_count
    }

    unsafe fn fail_size(&self) -> u32 {
        (*self.0.get()).fail_size.min(u32::MAX as usize) as u32
    }

    unsafe fn fail_align(&self) -> u32 {
        (*self.0.get()).fail_align.min(u32::MAX as usize) as u32
    }
}

#[global_allocator]
static ALLOCATOR: ResettableBumpAllocator = ResettableBumpAllocator::new();

unsafe impl GlobalAlloc for ResettableBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_from_arena(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = (ptr, layout);
    }
}

/// Capture panic message into PANIC_BUF for host-side diagnostics.
/// No heap allocation: writes directly into the static buffer.
#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    struct BufWriter {
        buf: &'static mut [u8],
        len: usize,
    }

    impl core::fmt::Write for BufWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            let remaining = self.buf.len().saturating_sub(self.len);
            let copy = bytes.len().min(remaining);
            if copy > 0 {
                self.buf[self.len..self.len + copy].copy_from_slice(&bytes[..copy]);
                self.len += copy;
            }
            Ok(())
        }
    }

    unsafe {
        let buf = core::slice::from_raw_parts_mut(PANIC_BUF.as_mut_ptr(), PANIC_BUF.len());
        let mut w = BufWriter { buf, len: 0 };
        let _ = core::fmt::write(&mut w, format_args!("{info}"));
        PANIC_LEN = w.len as u32;
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

struct PolkaVmSyscallProvider;

impl SyscallProvider for PolkaVmSyscallProvider {
    fn on_instruction(&mut self, opcode: u8) -> Result<(), alloc::string::String> {
        let success = unsafe { host_on_instruction(opcode as u32) };
        if success == 0 {
            return Err("host instruction charge failed".into());
        }
        Ok(())
    }

    fn syscall(
        &mut self,
        api: u32,
        ip: usize,
        stack: &mut Vec<StackValue>,
    ) -> Result<(), alloc::string::String> {
        unsafe {
            TRACE_SYSCALL_STAGE = 1;
            TRACE_SYSCALL_API = api;
            TRACE_SYSCALL_IP = ip.min(u32::MAX as usize) as u32;
            TRACE_STACK_ITEMS = stack.len().min(u32::MAX as usize) as u32;
            TRACE_REQ_LEN = 0;
        }
        let req_bytes = unsafe { REQ_BUF.as_mut_slice(REQ_BUF.len()) };
        req_bytes.fill(0);
        let req_bytes =
            postcard::to_slice(stack, req_bytes).map_err(|_| "failed to serialize stack")?;
        unsafe {
            TRACE_SYSCALL_STAGE = 2;
            TRACE_REQ_LEN = req_bytes.len().min(u32::MAX as usize) as u32;
        }

        unsafe {
            TRACE_SYSCALL_STAGE = 3;
        }
        let success = unsafe {
            host_call(
                api,
                ip as u32,
                req_bytes.as_ptr() as u32,
                req_bytes.len() as u32,
                RES_BUF.as_mut_ptr() as u32,
                RES_BUF.len() as u32,
            )
        };
        unsafe {
            TRACE_SYSCALL_STAGE = 4;
        }

        if success == 0 {
            return Err("host syscall failed".into());
        }

        let res_len = success as usize;
        unsafe {
            TRACE_RES_LEN = res_len as u32;
        }
        if res_len == 0 {
            *stack = Vec::new();
            return Ok(());
        }
        if res_len > RES_BUF.len() {
            return Err(alloc::format!("host response too large: {res_len}"));
        }
        unsafe {
            let res_bytes = RES_BUF.as_mut_slice(res_len);
            let new_stack = callback_codec::decode_stack_result(res_bytes)
                .map_err(|error| alloc::format!("failed to decode stack result: {error}"))??;
            let traced = callback_codec::encode_stack_result(&Ok(new_stack.clone()));
            let trace_head = TRACE_RES_HEAD.as_mut_slice(TRACE_HEAD_SIZE);
            let copy_len = core::cmp::min(traced.len(), TRACE_HEAD_SIZE);
            trace_head[..copy_len].copy_from_slice(&traced[..copy_len]);
            if copy_len < TRACE_HEAD_SIZE {
                trace_head[copy_len..].fill(0);
            }
            let retired = core::mem::replace(stack, new_stack);
            core::mem::forget(retired);
            TRACE_SYSCALL_STAGE = 5;
        }
        Ok(())
    }
}

static RESULT_BYTES: StaticVec = StaticVec::new();
static PANIC_BUF: RawBuffer<PANIC_BUF_SIZE> = RawBuffer::new();
static mut PANIC_LEN: u32 = 0;
static mut TRACE_RES_LEN: u32 = 0;
static TRACE_RES_HEAD: RawBuffer<TRACE_HEAD_SIZE> = RawBuffer::new();
static mut TRACE_SYSCALL_STAGE: u32 = 0;
static mut TRACE_SYSCALL_API: u32 = 0;
static mut TRACE_SYSCALL_IP: u32 = 0;
static mut TRACE_REQ_LEN: u32 = 0;
static mut TRACE_STACK_ITEMS: u32 = 0;

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_result_ptr() -> *const u8 {
    unsafe { RESULT_BYTES.as_ptr() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_result_len() -> u32 {
    unsafe { RESULT_BYTES.len() }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_panic_ptr() -> *const u8 {
    unsafe { PANIC_BUF.as_mut_ptr() as *const u8 }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_panic_len() -> u32 {
    unsafe { PANIC_LEN }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_res_len() -> u32 {
    unsafe { TRACE_RES_LEN }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_res_head_ptr() -> *const u8 {
    unsafe { TRACE_RES_HEAD.as_mut_ptr() as *const u8 }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_stage() -> u32 {
    unsafe { TRACE_SYSCALL_STAGE }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_api() -> u32 {
    unsafe { TRACE_SYSCALL_API }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_syscall_ip() -> u32 {
    unsafe { TRACE_SYSCALL_IP }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_req_len() -> u32 {
    unsafe { TRACE_REQ_LEN }
}

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn get_trace_stack_items() -> u32 {
    unsafe { TRACE_STACK_ITEMS }
}

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
    let result = execute_inner(script_ptr, script_len, stack_ptr, stack_len, initial_ip);
    unsafe {
        // Serialize result; if serialization fails (e.g., allocator exhaustion),
        // store a serialized error so the host gets a meaningful fault message.
        let bytes = postcard::to_allocvec(&result).unwrap_or_else(|_| {
            let err: Result<neo_riscv_abi::ExecutionResult, alloc::string::String> =
                Err(alloc::string::String::from("result serialization failed"));
            postcard::to_allocvec(&err).unwrap_or_default()
        });
        RESULT_BYTES.replace(bytes);
    }
}

fn execute_inner(
    script_ptr: u32,
    script_len: u32,
    stack_ptr: u32,
    stack_len: u32,
    initial_ip: u32,
) -> Result<ExecutionResult, alloc::string::String> {
    unsafe {
        ALLOCATOR.reset();
        RESULT_BYTES.replace(Vec::new());
        PANIC_LEN = 0;
        TRACE_SYSCALL_STAGE = 0;
        TRACE_SYSCALL_API = 0;
        TRACE_SYSCALL_IP = 0;
        TRACE_REQ_LEN = 0;
        TRACE_STACK_ITEMS = 0;
        TRACE_RES_LEN = 0;
        TRACE_RES_HEAD.as_mut_slice(TRACE_HEAD_SIZE).fill(0);
    }
    let script =
        unsafe { core::slice::from_raw_parts(script_ptr as *const u8, script_len as usize) };

    let initial_stack: Vec<StackValue> = if stack_len > 0 {
        let stack_bytes =
            unsafe { core::slice::from_raw_parts(stack_ptr as *const u8, stack_len as usize) };
        postcard::from_bytes(stack_bytes)
            .map_err(|e| alloc::format!("failed to deserialize initial stack: {e}"))?
    } else {
        Vec::new()
    };

    let mut provider = PolkaVmSyscallProvider;
    neo_riscv_guest::interpret_with_stack_and_syscalls_at(
        script,
        initial_stack,
        initial_ip as usize,
        &mut provider,
    )
}
