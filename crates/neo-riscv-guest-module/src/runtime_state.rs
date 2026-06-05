use super::{PANIC_BUF_SIZE, TRACE_HEAD_SIZE, bump_state::BumpState};

#[repr(C)]
pub(crate) struct RuntimeState {
    pub(crate) magic: u32,
    pub(crate) bump: BumpState,
    pub(crate) result_ptr: u32,
    pub(crate) result_len: u32,
    pub(crate) panic_len: u32,
    pub(crate) trace_res_len: u32,
    pub(crate) trace_syscall_stage: u32,
    pub(crate) trace_syscall_api: u32,
    pub(crate) trace_syscall_ip: u32,
    pub(crate) trace_req_len: u32,
    pub(crate) trace_stack_items: u32,
    pub(crate) trace_res_head: [u8; TRACE_HEAD_SIZE],
    pub(crate) panic_buf: [u8; PANIC_BUF_SIZE],
}

impl RuntimeState {
    pub(crate) const fn new() -> Self {
        Self {
            magic: 0x4e52_5653,
            bump: BumpState::new(),
            result_ptr: 0,
            result_len: 0,
            panic_len: 0,
            trace_res_len: 0,
            trace_syscall_stage: 0,
            trace_syscall_api: 0,
            trace_syscall_ip: 0,
            trace_req_len: 0,
            trace_stack_items: 0,
            trace_res_head: [0; TRACE_HEAD_SIZE],
            panic_buf: [0; PANIC_BUF_SIZE],
        }
    }
}
