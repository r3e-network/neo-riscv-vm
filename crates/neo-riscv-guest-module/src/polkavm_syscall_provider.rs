//! Guest-side syscall bridge between the `neo-vm-rs` interpreter and the PolkaVM host.
//!
//! This module implements [`SyscallProvider`] for the PolkaVM guest. The NeoVM
//! compatibility interpreter (`neo_riscv_guest`) calls back into this provider
//! on every opcode metering tick (`on_instruction`) and on every interop
//! syscall (`syscall`). Both are forwarded across the PolkaVM host-import
//! boundary to the Rust host (`neo-riscv-host::bridge`), which dispatches them
//! to the canonical NEO `System.*` interop handlers.
//!
//! # Syscall protocol (5 stages)
//!
//! A single [`SyscallProvider::syscall`](neo_riscv_guest::SyscallProvider::syscall)
//! call proceeds through five traceable stages, each recorded in
//! [`RuntimeState::trace_syscall_stage`](../runtime_state/struct.RuntimeState.html)
//! for post-mortem diagnostics:
//!
//! 1. **Record metadata** — store the `api` id, instruction pointer `ip`, and
//!    current stack depth so a fault can be attributed to the right call.
//! 2. **Encode request** — serialize the evaluation stack into the request
//!    arena (`REQ_BUF_OFFSET`) using the `fast_codec`.
//! 3. **Dispatch** — call the `host_call` host-import with the api id, request
//!    pointer/length, and the response arena (`RES_BUF_OFFSET`).
//! 4. **Receive** — the host returns the response byte count (`success`); zero
//!    means the syscall failed.
//! 5. **Decode response** — decode the response arena via `callback_codec`,
//!    replace the stack, and copy the first `TRACE_HEAD_SIZE` response bytes
//!    into the trace head for cheap inspection.
//!
//! `initializer_complete` reuses the same path with the
//! [`INITIALIZER_COMPLETE_MARKER`](neo_riscv_guest::INITIALIZER_COMPLETE_MARKER)
//! pseudo-api to signal that a `_initialize` routine finished.

use alloc::vec::Vec;

use neo_riscv_abi::{StackValue, callback_codec, fast_codec};
use neo_riscv_guest::SyscallProvider;

use super::{
    REQ_BUF_OFFSET, RES_BUF_OFFSET, SCRATCH_BUF_SIZE, TRACE_HEAD_SIZE, arena_ptr, arena_slice,
    host_call, host_on_instruction, runtime_state,
};

pub(crate) struct PolkaVmSyscallProvider;

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
            let state = runtime_state();
            state.trace_syscall_stage = 1;
            state.trace_syscall_api = api;
            state.trace_syscall_ip = ip.min(u32::MAX as usize) as u32;
            state.trace_stack_items = stack.len().min(u32::MAX as usize) as u32;
            state.trace_req_len = 0;
        }
        let req_bytes = unsafe { arena_slice(REQ_BUF_OFFSET, SCRATCH_BUF_SIZE) };
        let req_bytes = fast_codec::encode_stack_to_slice(stack, req_bytes)
            .map_err(|e| alloc::format!("failed to serialize stack: {e}"))?;
        unsafe {
            let state = runtime_state();
            state.trace_syscall_stage = 2;
            state.trace_req_len = req_bytes.len().min(u32::MAX as usize) as u32;
        }

        unsafe {
            runtime_state().trace_syscall_stage = 3;
        }
        let success = unsafe {
            host_call(
                api,
                ip as u32,
                req_bytes.as_ptr() as u32,
                req_bytes.len() as u32,
                arena_ptr(RES_BUF_OFFSET) as u32,
                SCRATCH_BUF_SIZE as u32,
            )
        };
        unsafe {
            runtime_state().trace_syscall_stage = 4;
        }

        if success == 0 {
            // Reset the response trace fields so a diagnostic dump after this
            // failure does not show stale data from a previous successful
            // syscall. `trace_syscall_stage` is intentionally left at the
            // failure stage (3) so the failure is still attributable.
            unsafe {
                let state = runtime_state();
                state.trace_res_len = 0;
                state.trace_res_head.fill(0);
            }
            return Err("host syscall failed".into());
        }

        let res_len = success as usize;
        unsafe {
            runtime_state().trace_res_len = res_len as u32;
        }
        if res_len == 0 {
            *stack = Vec::new();
            return Ok(());
        }
        if res_len > SCRATCH_BUF_SIZE {
            return Err(alloc::format!("host response too large: {res_len}"));
        }
        unsafe {
            let res_bytes = arena_slice(RES_BUF_OFFSET, res_len);

            let new_stack = callback_codec::decode_stack_result(res_bytes)
                .map_err(|error| alloc::format!("failed to decode stack result: {error}"))??;
            // Write trace header directly from response bytes (avoid clone which doubles memory).
            let trace_head = &mut runtime_state().trace_res_head[..];
            let copy_len = core::cmp::min(res_len, TRACE_HEAD_SIZE);
            trace_head[..copy_len].copy_from_slice(&res_bytes[..copy_len]);
            if copy_len < TRACE_HEAD_SIZE {
                trace_head[copy_len..].fill(0);
            }
            let retired = core::mem::replace(stack, new_stack);
            core::mem::forget(retired);
            runtime_state().trace_syscall_stage = 5;
        }
        Ok(())
    }

    fn initializer_complete(&mut self, ip: usize) -> Result<(), alloc::string::String> {
        let mut stack = Vec::new();
        self.syscall(neo_riscv_guest::INITIALIZER_COMPLETE_MARKER, ip, &mut stack)
    }
}
