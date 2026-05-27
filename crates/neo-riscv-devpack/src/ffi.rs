use alloc::{format, string::String, vec, vec::Vec};
use neo_riscv_abi::{callback_codec, StackValue};

unsafe extern "C" {
    fn host_call(
        api: u32,
        ip: u32,
        stack_ptr: usize,
        stack_len: usize,
        result_ptr: usize,
        result_cap: usize,
    ) -> usize;
}

pub fn invoke_host_call(api: u32, stack: &[StackValue]) -> Result<Vec<StackValue>, String> {
    let encoded = callback_codec::encode_stack_result(&Ok(stack.to_vec()));
    let mut result_buf = vec![0u8; 65536];

    let len = unsafe {
        host_call(
            api,
            0,
            encoded.as_ptr() as usize,
            encoded.len(),
            result_buf.as_mut_ptr() as usize,
            result_buf.capacity(),
        )
    };

    if len == 0 {
        // host_call returns 0 exclusively on failure (buffer-too-small, read failure,
        // write failure, deserialization failure). A successful empty stack encodes as
        // [2] (ENCODED_OK_EMPTY), which has length 1, not 0. See bridge.rs
        // host_call_import for the failure paths.
        return Err(format!("host_call(api=0x{api:08x}) failed: returned 0"));
    }

    result_buf.truncate(len as usize);
    callback_codec::decode_stack_result(&result_buf)?
}
