use std::cell::Cell;

thread_local! {
    /// Instruction pointer (NEF script offset) of the most recent FAULT on this
    /// thread, or `u32::MAX` if no attributed IP is available (HALT or FAULT
    /// without IP). Set from the fault paths in `execute_script_with_host_and_stack_and_ip`
    /// and siblings; retrieved via the `neo_riscv_last_fault_ip` FFI export.
    ///
    /// Side-channel design: avoids extending the shared `NativeExecutionResult`
    /// struct layout, which regressed multi-test sequences in C# P/Invoke.
    static LAST_FAULT_IP: Cell<u32> = const { Cell::new(u32::MAX) };

    /// Fast-codec-serialized locals snapshot of the faulting frame, retrievable via
    /// `neo_riscv_last_fault_locals` so the C# adapter can populate
    /// `ExecutionContext.LocalVariables` for dev-time introspection of faulted state.
    static LAST_FAULT_LOCALS: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };

    /// Native PolkaVM instruction fee consumed by the most recent direct
    /// contract execution on this thread. This lets FFI error paths report the
    /// fee even when execution aborts before producing an ExecutionResult.
    static LAST_NATIVE_FEE_CONSUMED_PICO: Cell<i64> = const { Cell::new(0) };
}

pub(crate) fn set_last_fault_ip(ip: Option<u32>) {
    LAST_FAULT_IP.with(|cell| cell.set(ip.unwrap_or(u32::MAX)));
}

pub(crate) fn reset_last_fault_ip() {
    LAST_FAULT_IP.with(|cell| cell.set(u32::MAX));
    LAST_FAULT_LOCALS.with(|cell| cell.borrow_mut().clear());
}

pub(crate) fn last_fault_ip() -> u32 {
    LAST_FAULT_IP.with(|cell| cell.get())
}

pub(crate) fn set_last_fault_locals(bytes: &Option<Vec<u8>>) {
    LAST_FAULT_LOCALS.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        if let Some(ref b) = *bytes {
            buf.extend_from_slice(b);
        }
    });
}

pub(crate) fn reset_last_native_fee_consumed_pico() {
    LAST_NATIVE_FEE_CONSUMED_PICO.with(|cell| cell.set(0));
}

pub(crate) fn set_last_native_fee_consumed_pico(value: i64) {
    LAST_NATIVE_FEE_CONSUMED_PICO.with(|cell| cell.set(value));
}

pub(crate) fn last_native_fee_consumed_pico() -> i64 {
    LAST_NATIVE_FEE_CONSUMED_PICO.with(|cell| cell.get())
}

/// Copies the most recently captured fault-locals byte buffer into the caller's
/// buffer. Returns the number of bytes available. If `out_capacity` is smaller
/// than the available length, no bytes are written (callers should call with
/// `out_capacity = 0` first to size their buffer, then allocate and re-call).
pub(crate) fn read_last_fault_locals(out_ptr: *mut u8, out_capacity: usize) -> usize {
    LAST_FAULT_LOCALS.with(|cell| {
        let buf = cell.borrow();
        let len = buf.len();
        if out_capacity >= len && !out_ptr.is_null() && len > 0 {
            // SAFETY: out_ptr is non-null, out_capacity >= len (checked above),
            // buf.as_ptr() is valid for len bytes (RefCell borrow guarantees
            // no mutation during the copy), and the two regions cannot overlap
            // (buf is owned by the thread-local RefCell, out_ptr is caller-allocated).
            unsafe {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), out_ptr, len);
            }
        }
        len
    })
}
