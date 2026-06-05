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
