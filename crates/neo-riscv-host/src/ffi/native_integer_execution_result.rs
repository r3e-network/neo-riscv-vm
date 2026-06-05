#[repr(C)]
pub struct NativeIntegerExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: u32,
    pub value: i64,
    pub error_ptr: *const u8,
    pub error_len: usize,
}
