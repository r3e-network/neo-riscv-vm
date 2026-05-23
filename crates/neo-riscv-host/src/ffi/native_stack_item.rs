#[repr(C)]
pub struct NativeStackItem {
    pub kind: u32,
    pub integer_value: i64,
    pub bytes_ptr: *mut u8,
    pub bytes_len: usize,
}
