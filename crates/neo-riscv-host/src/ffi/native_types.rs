//! `#[repr(C)]` structs shared across the FFI boundary with the .NET adapter.
//!
//! These mirror the `[StructLayout(LayoutKind.Sequential)]` declarations in
//! `NativeRiscvVmBridge.cs` and must stay layout-compatible. The C# side
//! marshals them via P/Invoke, so field order, types, and padding are an ABI.

/// A single NEO stack item marshalled across the FFI boundary.
///
/// This is the C-ABI representation of a `neo-vm-rs` [`StackValue`]. The `kind`
/// field is a discriminant that selects which of the payload fields is valid:
///
/// | `kind` | Variant | Payload field |
/// |--------|---------|---------------|
/// | 0 | Integer | `integer_value` |
/// | 1 | ByteString | `bytes_ptr`/`bytes_len` |
/// | 2 | Null | (none) |
/// | 3 | Boolean | `integer_value` (0/1) |
/// | 4 | Buffer | `bytes_ptr`/`bytes_len` |
/// | 5 | Pointer | `integer_value` |
/// | 6 | Array | `bytes_ptr`/`bytes_len` (encoded children) |
/// | 7 | Struct | `bytes_ptr`/`bytes_len` (encoded children) |
/// | 8 | Map | `bytes_ptr`/`bytes_len` (encoded entries) |
///
/// [`StackValue`]: neo_riscv_abi::StackValue
#[repr(C)]
pub struct NativeStackItem {
    /// Discriminant selecting the active payload (see the table in the type docs).
    pub kind: u32,
    /// Integer/Boolean payload (used when `kind` is Integer, Boolean, or Pointer).
    pub integer_value: i64,
    /// Pointer to the byte payload (ByteString/Buffer/Array/Struct/Map); owned by the Rust side.
    pub bytes_ptr: *const u8,
    /// Length in bytes of the payload at `bytes_ptr`.
    pub bytes_len: usize,
}

/// Result of an execution that produces a single integer value (e.g. a native
/// contract returning an `i64`).
///
/// Lightweight alternative to [`NativeExecutionResult`] for the common
/// single-integer case, avoiding a stack allocation on the C# side.
#[repr(C)]
pub struct NativeIntegerExecutionResult {
    /// PolkaVM instruction fee consumed, in pico units.
    pub fee_consumed_pico: i64,
    /// Terminal VM state (halt/fault), matching [`VmState`].
    ///
    /// [`VmState`]: neo_riscv_abi::VmState
    pub state: u32,
    /// The integer result value.
    pub value: i64,
    /// Pointer to a UTF-8 error message (valid when `state` indicates a fault).
    pub error_ptr: *const u8,
    /// Length of the error message at `error_ptr`.
    pub error_len: usize,
}

/// Outcome of a host-import callback invocation, returned from the Rust host to
/// the C# adapter.
///
/// Carries either a replacement evaluation stack (on success) or an error
/// message (on failure). The stack array is owned by the Rust side and must be
/// freed via the matching free function.
#[repr(C)]
pub struct NativeHostResult {
    /// Pointer to the returned stack array (`stack_len` items).
    pub stack_ptr: *mut NativeStackItem,
    /// Number of items in the stack array.
    pub stack_len: usize,
    /// Pointer to a UTF-8 error message (null/zero-length on success).
    pub error_ptr: *const u8,
    /// Length of the error message at `error_ptr`.
    pub error_len: usize,
}

/// Full result of a script or contract execution, marshalled across the FFI
/// boundary to the .NET adapter.
///
/// This is the primary return type of the `neo_riscv_execute_script*` and
/// `neo_riscv_execute_native_contract*` FFI exports. The `stack_ptr` array is
/// Rust-owned and must be released with `neo_riscv_free_execution_result`.
#[repr(C)]
pub struct NativeExecutionResult {
    /// PolkaVM instruction fee consumed, in pico units.
    pub fee_consumed_pico: i64,
    /// Terminal VM state (halt/fault), matching [`VmState`].
    ///
    /// [`VmState`]: neo_riscv_abi::VmState
    pub state: u32,
    /// Pointer to the resulting evaluation stack array (`stack_len` items).
    pub stack_ptr: *mut NativeStackItem,
    /// Number of items in the stack array.
    pub stack_len: usize,
    /// Pointer to a UTF-8 error message (valid when `state` indicates a fault).
    pub error_ptr: *const u8,
    /// Length of the error message at `error_ptr`.
    pub error_len: usize,
}
