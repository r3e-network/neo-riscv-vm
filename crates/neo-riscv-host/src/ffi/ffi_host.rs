use std::{ffi::c_void, ptr};

use crate::{pricing::charge_opcode, RuntimeContext};
use neo_riscv_guest::SyscallProvider;

use super::{
    copy_native_host_result, serialize_stack_items_fast, NativeHostCallback,
    NativeHostFreeCallback, NativeHostResult,
};

pub(super) struct FfiHost {
    pub(super) context: RuntimeContext,
    pub(super) fee_consumed_pico: i64,
    pub(super) user_data: *mut c_void,
    pub(super) callback: NativeHostCallback,
    pub(super) free_callback: NativeHostFreeCallback,
}

impl SyscallProvider for FfiHost {
    fn on_instruction(&mut self, opcode: u8) -> Result<(), String> {
        charge_opcode(&mut self.context, &mut self.fee_consumed_pico, opcode)
    }

    fn syscall(
        &mut self,
        api: u32,
        ip: usize,
        stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        *stack = self.syscall_host(api, ip, stack)?;
        Ok(())
    }
}

impl FfiHost {
    pub(super) fn syscall_host(
        &mut self,
        api: u32,
        ip: usize,
        stack: &[neo_riscv_abi::StackValue],
    ) -> Result<Vec<neo_riscv_abi::StackValue>, String> {
        let mut result = NativeHostResult {
            stack_ptr: ptr::null_mut(),
            stack_len: 0,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
        let serialized_stack = serialize_stack_items_fast(stack);
        let input_stack_ptr = serialized_stack.ptr();
        let input_stack_len = serialized_stack.len();

        // SAFETY: self.callback is a C# delegate pointer provided by the FFI caller.
        // self.user_data is a GCHandle pinned for the duration of the execution.
        // input_stack_ptr is valid until the serialized stack is freed below.
        let invoked = unsafe {
            (self.callback)(
                self.user_data,
                api,
                ip,
                self.context.trigger,
                self.context.network,
                self.context.address_version,
                self.context.timestamp.unwrap_or_default(),
                self.context.gas_left,
                input_stack_ptr,
                input_stack_len,
                &mut result,
            )
        };
        serialized_stack.free();

        if !invoked {
            return Err(format!(
                "host callback invocation failed for syscall 0x{api:08x}"
            ));
        }

        let host_result = copy_native_host_result(&result);
        // SAFETY: self.free_callback is the C# cleanup delegate. result was populated by
        // the callback above and is valid for this call.
        unsafe {
            (self.free_callback)(self.user_data, &mut result);
        }

        let host_result = host_result?;
        Ok(host_result.stack)
    }
}
