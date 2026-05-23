use super::NativeStackItem;

#[repr(C)]
pub struct NativeHostResult {
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *mut u8,
    pub error_len: usize,
}
