//! `#[repr(C)]` structs shared across the FFI boundary with the .NET adapter.

#[repr(C)]
pub struct NativeStackItem {
    pub kind: u32,
    pub integer_value: i64,
    pub bytes_ptr: *const u8,
    pub bytes_len: usize,
}

#[repr(C)]
pub struct NativeIntegerExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: u32,
    pub value: i64,
    pub error_ptr: *const u8,
    pub error_len: usize,
}

#[repr(C)]
pub struct NativeHostResult {
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *const u8,
    pub error_len: usize,
}

#[repr(C)]
pub struct NativeExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: u32,
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *const u8,
    pub error_len: usize,
}
