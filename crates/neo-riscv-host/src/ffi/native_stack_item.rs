#[repr(C)]
pub struct NativeStackItem {
    pub kind: u32,
    pub integer_value: i64,
    pub bytes_ptr: *const u8,
    pub bytes_len: usize,
}
