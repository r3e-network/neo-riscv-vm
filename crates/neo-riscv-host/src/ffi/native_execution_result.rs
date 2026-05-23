use super::NativeStackItem;

#[repr(C)]
pub struct NativeExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: u32,
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *mut u8,
    pub error_len: usize,
}
