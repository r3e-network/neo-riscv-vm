//! Neo RISC-V host runtime.
//!
//! Provides the host-side VM runtime using PolkaVM, FFI bindings for C# interop,
//! and execution context management.

// Require every public item to carry rustdoc. This prevents documentation debt
// from silently accumulating on the host's public API surface.
#![deny(missing_docs)]

mod bridge;
/// Typed error variants returned by the host runtime ([`HostError`]).
pub mod error;
mod ffi;
mod pricing;
mod profiling;
mod runtime_cache;
mod types;

use bridge::{
    ClosureHost, GuestTrace, read_guest_debug, read_guest_last_interpreter_ip, read_guest_panic,
    read_guest_result_diag, read_guest_trace,
};
use neo_riscv_abi::{BackendKind, ExecutionResult, StackValue, VmState, fast_codec};
use serde::Deserialize;
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

/// Reset all execution thread-local side-channels at the start of an execution.
///
/// This must be called at the top of every `execute_*` entry point so that a
/// HALT following a previous FAULT on the same thread does not leak the prior
/// fault's IP/locals, and so the native-fee counter starts from zero. Without
/// this, `neo_riscv_last_fault_ip()` could return a stale IP from an unrelated
/// earlier execution.
pub(crate) fn reset_execution_sidechannels() {
    reset_last_fault_ip();
    reset_last_native_fee_consumed_pico();
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
            // buf.as_ptr() is valid for len bytes, and the two regions cannot
            // overlap (buf is thread-local, out_ptr is caller-allocated).
            unsafe {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), out_ptr, len);
            }
        }
        len
    })
}

/// Maximum allowed result size from guest (16 MB).
const MAX_RESULT_SIZE: u32 = 16 * 1024 * 1024;

#[derive(Deserialize)]
enum LegacyPostcardVmState {
    Halt,
    Fault,
}

#[derive(Deserialize)]
struct LegacyPostcardExecutionResult {
    fee_consumed_pico: i64,
    state: LegacyPostcardVmState,
    stack: Vec<StackValue>,
    #[serde(default)]
    fault_message: Option<String>,
    #[serde(default)]
    fault_ip: Option<u32>,
    #[serde(default)]
    fault_locals: Option<Vec<u8>>,
}

impl From<LegacyPostcardExecutionResult> for ExecutionResult {
    fn from(result: LegacyPostcardExecutionResult) -> Self {
        Self {
            fee_consumed_pico: result.fee_consumed_pico,
            state: match result.state {
                LegacyPostcardVmState::Halt => VmState::Halt,
                LegacyPostcardVmState::Fault => VmState::Fault,
            },
            stack: result.stack,
            fault_message: result.fault_message,
            fault_ip: result.fault_ip,
            fault_locals: result.fault_locals,
        }
    }
}

fn decode_guest_execution_result(bytes: &[u8]) -> Result<Result<ExecutionResult, String>, String> {
    match neo_riscv_abi::result_codec::decode_execution_result(bytes) {
        Ok(result) => Ok(result),
        Err(shared_error) => {
            postcard::from_bytes::<Result<LegacyPostcardExecutionResult, String>>(bytes)
                .map(|result| result.map(Into::into))
                .map_err(|legacy_error| {
                    format!(
                        "Failed to decode result: shared codec: {shared_error}; legacy postcard: {legacy_error}"
                    )
                })
        }
    }
}

pub use ffi::{
    NativeExecutionResult, NativeHostCallback, NativeHostFreeCallback, NativeHostResult,
    NativeStackItem, neo_riscv_execute_native_contract, neo_riscv_execute_native_contract_builtin,
    neo_riscv_execute_native_contract_builtin_by_id,
    neo_riscv_execute_native_contract_builtin_i64_by_id, neo_riscv_execute_script,
    neo_riscv_execute_script_with_host, neo_riscv_execute_script_with_host_and_initializer,
    neo_riscv_execute_script_with_host_and_initializer_and_result_limit,
    neo_riscv_execute_script_with_host_and_result_limit, neo_riscv_free_execution_result,
};
pub use profiling::{get_current_memory, get_peak_memory, reset as reset_profiling};
pub use types::{HostCallbackResult, RuntimeContext};

/// A handle to the cached PolkaVM execution backend.
///
/// Wraps the lazily-initialized PolkaVM module cache. Construct once with
/// [`PolkaVmRuntime::new`] and reuse for many executions; the underlying guest
/// module is compiled and cached on first use.
pub struct PolkaVmRuntime {
    backend_kind: BackendKind,
}

impl PolkaVmRuntime {
    /// Create a new runtime, initializing the PolkaVM module cache if needed.
    ///
    /// # Errors
    /// Returns an error string if the PolkaVM guest module cannot be loaded or
    /// compiled (e.g. missing built-in binary, allocation failure).
    pub fn new() -> Result<Self, String> {
        runtime_cache::ensure_runtime_ready()?;

        Ok(Self {
            backend_kind: BackendKind::Interpreter,
        })
    }

    /// Return which PolkaVM backend kind is active (currently always
    /// [`BackendKind::Interpreter`]).
    #[must_use]
    pub fn backend_kind(&self) -> BackendKind {
        self.backend_kind
    }
}

use error::HostError;

/// Execute a NeoVM script with a default `Application` trigger and zeroed
/// context fields.
///
/// Convenience wrapper around [`execute_script_with_context`] for the common
/// case of running a script standalone. Syscalls are served by the built-in
/// [`builtin_host_callback`].
pub fn execute_script(script: &[u8]) -> Result<ExecutionResult, HostError> {
    execute_script_with_context(
        script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
    )
}

/// Execute a NeoVM script with a custom trigger type (e.g. verification).
///
/// Like [`execute_script`] but lets the caller specify the trigger byte
/// (`0x40` Application, `0x20` Verification, etc.), which native contracts use
/// to decide whether an invocation is permitted.
pub fn execute_script_with_trigger(
    script: &[u8],
    trigger: u8,
) -> Result<ExecutionResult, HostError> {
    execute_script_with_context(
        script,
        RuntimeContext {
            trigger,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
    )
}

/// Execute a NeoVM script with a full runtime context (trigger, network,
/// address version, timestamp, gas, fee factor).
///
/// Syscalls (`System.*` interops) are dispatched to the built-in
/// [`builtin_host_callback`]. Use [`execute_script_with_host_and_stack`] when
/// you need to supply your own syscall handler (e.g. the C# adapter does, to
/// route syscalls back across the FFI boundary).
pub fn execute_script_with_context(
    script: &[u8],
    context: RuntimeContext,
) -> Result<ExecutionResult, HostError> {
    execute_script_with_host_and_stack_and_ip(
        script,
        Vec::new(),
        0,
        context,
        |api, _ip, context, stack| builtin_host_callback(api, context, stack),
    )
    .map_err(HostError::from)
}

/// Execute a NeoVM script with a caller-supplied syscall callback.
///
/// This is the primary entry point used by the C# adapter: every `System.*`
/// interop raised by the interpreter is forwarded to `callback`, which marshals
/// it back across the FFI boundary to the .NET host for canonical handling.
/// The initial evaluation stack and instruction pointer start empty/zero.
pub fn execute_script_with_host_and_stack<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack_and_ip(script, initial_stack, 0, context, callback)
}

#[doc(hidden)]
pub fn debug_execute_script_with_host_and_stack<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    context: RuntimeContext,
    mut callback: F,
) -> Result<(ExecutionResult, GuestTrace), String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    reset_execution_sidechannels();
    let script_len: u32 = script
        .len()
        .try_into()
        .map_err(|_| "script too large for u32".to_string())?;
    let initial_stack_bytes = fast_codec::encode_stack(&initial_stack);
    let stack_len: u32 = initial_stack_bytes
        .len()
        .try_into()
        .map_err(|_| "encoded stack too large for u32".to_string())?;
    let aux_size = required_aux_size(script_len, stack_len);
    let mut cached_instance = runtime_cache::cached_execution_instance(aux_size)?;
    let mut host = ClosureHost::new(context, &mut callback);
    let aux_base = cached_instance.module().memory_map().aux_data_address();
    let instance = cached_instance.instance_mut();
    let stack_offset = align_up_u32(script_len, 8);
    let script_ptr = if script_len > 0 { aux_base } else { 0 };
    let stack_ptr = if stack_len > 0 {
        aux_base
            .checked_add(stack_offset)
            .ok_or_else(|| "aux data offset overflow".to_string())?
    } else {
        0
    };

    if aux_size > 0 {
        instance
            .set_accessible_aux_size(aux_size)
            .map_err(|e| format!("guest aux setup failed: {e}"))?;
    }

    if script_len > 0 {
        instance
            .write_memory(script_ptr, script)
            .map_err(|e| format!("guest write_memory failed: {e:?}"))?;
    }

    if stack_len > 0 {
        instance
            .write_memory(stack_ptr, &initial_stack_bytes)
            .map_err(|e| format!("guest write_memory failed: {e:?}"))?;
    }

    instance
        .call_typed(
            &mut host,
            "execute",
            (script_ptr, script_len, stack_ptr, stack_len, 0u32),
        )
        .map_err(|e| {
            set_last_native_fee_consumed_pico(host.fee_consumed_pico);
            let trace = read_guest_trace(instance, &mut host);
            // Read guest panic message if available
            let panic_msg = read_guest_panic(instance, &mut host);
            let result_diag = read_guest_result_diag(instance, &mut host);
            let fault_ip = read_guest_last_interpreter_ip(instance, &mut host);
            {
                let alloc_peak = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ()).unwrap_or(0);
                let alloc_fails = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ()).unwrap_or(0);
                let alloc_fail_size = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ()).unwrap_or(0);
                format!(
                    "guest execute failed: {e:?}; last_opcode={:?}; last_guest_ip={fault_ip:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; result_diag={result_diag:?}; trace={trace:?}; panic={panic_msg:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
                    host.last_opcode,
                    host.opcode_count,
                    host.syscall_count,
                    host.last_api,
                    host.last_ip,
                    host.last_stack_len,
                    host.last_result_cap,
                    host.last_host_call_stage
                )
            }
        })?;

    let trace = read_guest_trace(instance, &mut host);

    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("guest get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("guest get_result_len failed: {e:?}"))?;

    if res_len > MAX_RESULT_SIZE {
        return Err(format!(
            "guest result size {res_len} exceeds maximum {MAX_RESULT_SIZE}"
        ));
    }
    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("guest read_memory failed: {e:?}"))?;
    let mut result = decode_guest_execution_result(&res_bytes)?;
    set_last_native_fee_consumed_pico(host.fee_consumed_pico);

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the guest returned a VM-level FAULT with fault_message, return it without
    // internal trace (trace is for FFI-level errors only).
    if let Ok(ref r) = result
        && r.state == VmState::Fault
    {
        // Capture IP and locals in thread-local side-channels before the Ok(Fault)→Err
        // conversion loses them. C# retrieves via `neo_riscv_last_fault_ip()` and
        // `neo_riscv_last_fault_locals()`.
        set_last_fault_ip(r.fault_ip);
        set_last_fault_locals(&r.fault_locals);
        // Prefer host.charge_error when the host-side charge failed (gas exhaustion or
        // instruction ceiling): the guest only saw the import's 0-return and replied
        // with a generic "host instruction charge failed". host.charge_error preserves
        // the specific reason.
        if let Some(ref msg) = host.charge_error {
            return Err(msg.clone());
        }
        if let Some(ref msg) = r.fault_message {
            return Err(msg.clone());
        }
    }

    match result {
        Ok(result) => Ok((result, trace)),
        Err(error) => {
            if let Some(ip) = read_guest_last_interpreter_ip(instance, &mut host) {
                set_last_fault_ip(Some(ip));
                Err(format!("{error}; ip={ip}; trace={trace:?}"))
            } else {
                Err(format!("{error}; trace={trace:?}"))
            }
        }
    }
}

/// Execute a NeoVM script from a non-zero instruction pointer.
///
/// Like [`execute_script_with_host_and_stack`] but starts interpretation at
/// `initial_ip` instead of offset 0 — used when resuming a partially-executed
/// script or entering at a specific method offset within a NEF script.
pub fn execute_script_with_host_and_stack_and_ip<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack_and_ip_inner(
        script,
        initial_stack,
        initial_ip,
        None,
        None,
        context,
        callback,
    )
}

/// Execute a NeoVM script, running an `_initialize` routine first.
///
/// The interpreter runs the script from `initializer_ip` to completion first
/// (the legacy deploy-time initializer), then continues from `initial_ip`.
/// Syscalls go through the caller-supplied `callback`.
pub fn execute_script_with_host_and_stack_and_ip_and_initializer<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
    initializer_ip: usize,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack_and_ip_inner(
        script,
        initial_stack,
        initial_ip,
        Some(initializer_ip),
        None,
        context,
        callback,
    )
}

/// Execute a NeoVM script, capping the returned stack to `result_limit` items.
///
/// When only the top of the result stack is needed (e.g. verification), this
/// avoids serializing the full stack back across the host/guest boundary.
pub fn execute_script_with_host_and_stack_and_ip_with_result_limit<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
    result_limit: usize,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack_and_ip_inner(
        script,
        initial_stack,
        initial_ip,
        None,
        Some(result_limit),
        context,
        callback,
    )
}

/// Execute a NeoVM script with both an `_initialize` routine and a result cap.
///
/// Combination of
/// [`execute_script_with_host_and_stack_and_ip_and_initializer`] and
/// [`execute_script_with_host_and_stack_and_ip_with_result_limit`]: runs the
/// initializer first, then the main entry, and caps the returned stack.
pub fn execute_script_with_host_and_stack_and_ip_and_initializer_with_result_limit<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
    initializer_ip: usize,
    result_limit: usize,
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack_and_ip_inner(
        script,
        initial_stack,
        initial_ip,
        Some(initializer_ip),
        Some(result_limit),
        context,
        callback,
    )
}

fn execute_script_with_host_and_stack_and_ip_inner<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
    initializer_ip: Option<usize>,
    result_limit: Option<usize>,
    context: RuntimeContext,
    mut callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    reset_execution_sidechannels();
    let script_len: u32 = script
        .len()
        .try_into()
        .map_err(|_| "script too large for u32".to_string())?;
    let initial_stack_bytes = fast_codec::encode_stack(&initial_stack);
    let stack_len: u32 = initial_stack_bytes
        .len()
        .try_into()
        .map_err(|_| "encoded stack too large for u32".to_string())?;
    let aux_size = required_aux_size(script_len, stack_len);
    let mut cached_instance = runtime_cache::cached_execution_instance(aux_size)?;
    let mut host = ClosureHost::new(context, &mut callback);
    let aux_base = cached_instance.module().memory_map().aux_data_address();
    let instance = cached_instance.instance_mut();
    let stack_offset = align_up_u32(script_len, 8);
    let script_ptr = if script_len > 0 { aux_base } else { 0 };
    let stack_ptr = if stack_len > 0 {
        aux_base
            .checked_add(stack_offset)
            .ok_or_else(|| "aux data offset overflow".to_string())?
    } else {
        0
    };

    if aux_size > 0 {
        instance
            .set_accessible_aux_size(aux_size)
            .map_err(|e| format!("guest aux setup failed: {e}"))?;
    }

    if script_len > 0 {
        instance
            .write_memory(script_ptr, script)
            .map_err(|e| format!("guest write_memory failed: {e:?}"))?;
    }

    if stack_len > 0 {
        instance
            .write_memory(stack_ptr, &initial_stack_bytes)
            .map_err(|e| format!("guest write_memory failed: {e:?}"))?;
    }

    if let (Some(_), Some(result_limit)) = (initializer_ip, result_limit) {
        instance
            .call_typed(&mut host, "set_result_limit", (result_limit as u32,))
            .map_err(|e| {
                set_last_native_fee_consumed_pico(host.fee_consumed_pico);
                format!("guest set_result_limit failed: {e:?}")
            })?;
    }

    let execution_call = match (initializer_ip, result_limit) {
        (Some(initializer_ip), Some(_)) | (Some(initializer_ip), None) => instance.call_typed(
            &mut host,
            "execute_with_initializer",
            (
                script_ptr,
                script_len,
                stack_ptr,
                stack_len,
                initial_ip as u32,
                initializer_ip as u32,
            ),
        ),
        (None, Some(result_limit)) => instance.call_typed(
            &mut host,
            "execute_with_result_limit",
            (
                script_ptr,
                script_len,
                stack_ptr,
                stack_len,
                initial_ip as u32,
                result_limit as u32,
            ),
        ),
        (None, None) => instance.call_typed(
            &mut host,
            "execute",
            (
                script_ptr,
                script_len,
                stack_ptr,
                stack_len,
                initial_ip as u32,
            ),
        ),
    };
    execution_call
        .map_err(|e| {
            set_last_native_fee_consumed_pico(host.fee_consumed_pico);
            let trace = read_guest_trace(instance, &mut host);
            let panic_msg = read_guest_panic(instance, &mut host);
            let result_diag = read_guest_result_diag(instance, &mut host);
            let fault_ip = read_guest_last_interpreter_ip(instance, &mut host);
            {
                let alloc_peak = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ()).unwrap_or(0);
                let alloc_fails = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ()).unwrap_or(0);
                let alloc_fail_size = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ()).unwrap_or(0);
                format!(
                    "guest execute failed: {e:?}; last_opcode={:?}; last_guest_ip={fault_ip:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; result_diag={result_diag:?}; trace={trace:?}; panic={panic_msg:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
                    host.last_opcode,
                    host.opcode_count,
                    host.syscall_count,
                    host.last_api,
                    host.last_ip,
                    host.last_stack_len,
                    host.last_result_cap,
                    host.last_host_call_stage
                )
            }
        })?;

    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("guest get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("guest get_result_len failed: {e:?}"))?;

    if res_len > MAX_RESULT_SIZE {
        return Err(format!(
            "guest result size {res_len} exceeds maximum {MAX_RESULT_SIZE}"
        ));
    }
    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("guest read_memory failed: {e:?}"))?;
    let mut result = decode_guest_execution_result(&res_bytes)?;
    set_last_native_fee_consumed_pico(host.fee_consumed_pico);

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the guest returned a VM-level FAULT with fault_message, return it without
    // internal trace (trace is for FFI-level errors only).
    if let Ok(ref r) = result
        && r.state == VmState::Fault
    {
        // Capture IP and locals in thread-local side-channels before the Ok(Fault)→Err
        // conversion loses them. C# retrieves via `neo_riscv_last_fault_ip()` and
        // `neo_riscv_last_fault_locals()`.
        set_last_fault_ip(r.fault_ip);
        set_last_fault_locals(&r.fault_locals);
        // Prefer host.charge_error when the host-side charge failed (gas exhaustion or
        // instruction ceiling): the guest only saw the import's 0-return and replied
        // with a generic "host instruction charge failed". host.charge_error preserves
        // the specific reason.
        if let Some(ref msg) = host.charge_error {
            return Err(msg.clone());
        }
        if let Some(ref msg) = r.fault_message {
            return Err(msg.clone());
        }
    }

    if let Err(error) = result {
        let trace = read_guest_trace(instance, &mut host);
        if let Some(ip) = read_guest_last_interpreter_ip(instance, &mut host) {
            set_last_fault_ip(Some(ip));
            return Err(format!("{error}; ip={ip}; trace={trace:?}"));
        }
        return Err(format!("{error}; trace={trace:?}"));
    }

    result
}

fn align_up_u32(value: u32, align: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.div_ceil(align).saturating_mul(align)
    }
}

fn required_aux_size(script_len: u32, stack_len: u32) -> u32 {
    if script_len == 0 && stack_len == 0 {
        0
    } else {
        align_up_u32(script_len, 8).saturating_add(stack_len)
    }
}

pub(crate) fn charge_native_metered_instructions(
    instance: &polkavm::Instance<ClosureHost, core::convert::Infallible>,
    host: &mut ClosureHost,
    gas_limit: i64,
) -> Result<(), String> {
    let gas_remaining = instance.gas().max(0);
    let instruction_count = gas_limit.saturating_sub(gas_remaining);
    let result = crate::pricing::charge_native_instructions(
        &mut host.context,
        &mut host.fee_consumed_pico,
        instruction_count,
    );
    set_last_native_fee_consumed_pico(host.fee_consumed_pico);
    result
}

pub(crate) fn native_call_error(error: &polkavm::CallError) -> Option<&'static str> {
    match error {
        polkavm::CallError::NotEnoughGas => Some("Insufficient GAS."),
        _ => None,
    }
}

/// Execute a native RISC-V contract binary directly via PolkaVM.
///
/// Unlike `execute_script_*` which runs NeoVM bytecode through the cached
/// compatibility contract, this compiles the contract binary itself as a PolkaVM
/// module and executes it directly. The contract must export `execute(u32, u32)`,
/// `get_result_ptr()`, and `get_result_len()` functions, and may import
/// `host_call` for syscalls.
///
/// The method name is prepended to `initial_stack` as a `ByteString` so the
/// contract can dispatch by method.
pub fn execute_native_contract<F>(
    binary: &[u8],
    method: &str,
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    context: RuntimeContext,
    mut callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    reset_execution_sidechannels();
    let stack_bytes = fast_codec::encode_stack(&initial_stack);

    // The fallback stack (method name + cloned initial stack) is only needed when
    // the module lacks an `execute_method` export. To size `aux_size` without
    // eagerly cloning the whole stack, compute a conservative upper bound on the
    // fallback encoding length: the encoded method name cannot exceed
    // method.len() + a few tag/length bytes, plus the already-known
    // stack_bytes length.
    let fallback_len_upper_bound = stack_bytes.len() + method.len() + 16;
    let max_stack_len = stack_bytes.len().max(fallback_len_upper_bound);
    let aux_size = if max_stack_len == 0 {
        0
    } else {
        align_up_u32(
            u32::try_from(max_stack_len)
                .map_err(|_| "native contract stack too large".to_string())?,
            8,
        )
    };

    let mut cached_instance = runtime_cache::cached_native_execution_instance(binary, aux_size)?;
    let aux_base = if !stack_bytes.is_empty() {
        cached_instance.module().memory_map().aux_data_address()
    } else {
        0
    };
    let has_execute_method = cached_instance
        .module()
        .exports()
        .any(|export| export.symbol().as_bytes() == b"execute_method");
    let instance = cached_instance.instance_mut();

    let mut host = bridge::ClosureHost::new(context, &mut callback);
    let native_gas_limit = crate::pricing::native_instruction_limit(&host.context);

    // Write serialized stack to aux memory
    let stack_ptr = if !stack_bytes.is_empty() {
        if aux_size > 0 {
            instance
                .set_accessible_aux_size(aux_size)
                .map_err(|e| format!("native aux setup failed: {e}"))?;
        }
        instance
            .write_memory(aux_base, &stack_bytes)
            .map_err(|e| format!("native write_memory failed: {e:?}"))?;
        aux_base
    } else {
        0
    };
    let stack_len: u32 = stack_bytes
        .len()
        .try_into()
        .map_err(|_| "native contract stack too large for u32".to_string())?;

    let method_id = native_method_id(method);
    instance.set_gas(native_gas_limit);
    let call_result = match instance.call_typed(
        &mut host,
        "execute_method",
        (method_id, stack_ptr, stack_len),
    ) {
        Ok(()) => Ok(()),
        Err(_) if !has_execute_method => {
            // Lazily build the fallback stack (method name + cloned initial
            // stack) only when the legacy `execute` export path is needed. This
            // avoids cloning every StackValue in the common case where the
            // module exports `execute_method`.
            let mut fallback_full_stack = Vec::with_capacity(1 + initial_stack.len());
            fallback_full_stack.push(neo_riscv_abi::StackValue::ByteString(
                method.as_bytes().to_vec(),
            ));
            fallback_full_stack.extend(initial_stack.iter().cloned());
            let fallback_stack_bytes = fast_codec::encode_stack(&fallback_full_stack);

            let full_stack_len: u32 = fallback_stack_bytes
                .len()
                .try_into()
                .map_err(|_| "native contract stack too large for u32".to_string())?;

            if aux_size > 0 {
                instance
                    .set_accessible_aux_size(aux_size)
                    .map_err(|e| format!("native aux setup failed: {e}"))?;
            }
            instance
                .write_memory(aux_base, &fallback_stack_bytes)
                .map_err(|e| format!("native write_memory failed: {e:?}"))?;

            instance.call_typed(&mut host, "execute", (stack_ptr, full_stack_len))
        }
        Err(error) => Err(error),
    };

    if let Err(error) = call_result {
        let charge_error =
            charge_native_metered_instructions(instance, &mut host, native_gas_limit).err();
        if let Some(message) = charge_error
            .as_deref()
            .or_else(|| native_call_error(&error).or(host.charge_error.as_deref()))
        {
            return Err(message.to_string());
        }
        instance.set_gas(crate::pricing::NEO_INSTRUCTION_CEILING as i64);
        let trace = read_guest_trace(instance, &mut host);
        let panic_msg = read_guest_panic(instance, &mut host);
        let debug = read_guest_debug(instance, &mut host);
        let alloc_peak = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ())
            .unwrap_or(0);
        let alloc_fails = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ())
            .unwrap_or(0);
        let alloc_fail_size = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ())
            .unwrap_or(0);
        return Err(format!(
            "native contract execute failed: {error:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}; panic={panic_msg:?}; debug={debug:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
            host.last_opcode,
            host.opcode_count,
            host.syscall_count,
            host.last_api,
            host.last_ip,
            host.last_stack_len,
            host.last_result_cap,
            host.last_host_call_stage
        ));
    }

    // Read result back
    let res_ptr: u32 =
        match instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ()) {
            Ok(value) => value,
            Err(error) => {
                charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;
                if let Some(message) = native_call_error(&error) {
                    return Err(message.to_string());
                }
                return Err(format!("native get_result_ptr failed: {error:?}"));
            }
        };
    let res_len: u32 =
        match instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ()) {
            Ok(value) => value,
            Err(error) => {
                charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;
                if let Some(message) = native_call_error(&error) {
                    return Err(message.to_string());
                }
                return Err(format!("native get_result_len failed: {error:?}"));
            }
        };

    if res_len > MAX_RESULT_SIZE {
        return Err(format!(
            "native result size {res_len} exceeds maximum {MAX_RESULT_SIZE}"
        ));
    }
    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("native read_memory failed: {e:?}"))?;

    charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;

    let mut result = neo_riscv_abi::result_codec::decode_execution_result(&res_bytes)
        .map_err(|_| "failed to decode native result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the contract returned a VM-level FAULT with fault_message, return as error
    // (consistent with execute_script_* paths).
    if let Ok(ref r) = result
        && r.state == VmState::Fault
    {
        // Capture IP and locals in thread-local side-channels before the Ok(Fault)→Err
        // conversion loses them. C# retrieves via `neo_riscv_last_fault_ip()` and
        // `neo_riscv_last_fault_locals()`.
        set_last_fault_ip(r.fault_ip);
        set_last_fault_locals(&r.fault_locals);
        // Prefer host.charge_error when the host-side charge failed (gas exhaustion or
        // instruction ceiling): the guest only saw the import's 0-return and replied
        // with a generic "host instruction charge failed". host.charge_error preserves
        // the specific reason.
        if let Some(ref msg) = host.charge_error {
            return Err(msg.clone());
        }
        if let Some(ref msg) = r.fault_message {
            return Err(msg.clone());
        }
    }

    result
}

/// Compute the native contract method dispatch ID using FNV-1a (32-bit).
///
/// **Security**: 32-bit FNV-1a has a birthday bound of ~77K names. This is
/// acceptable because native contract method names come from a fixed,
/// controlled set defined in each native contract's source — not user input.
/// The guest-side syscall dispatcher uses `interop_hash` (SHA-256 truncated)
/// for its broader, partly user-controlled API surface.
///
/// **Migration**: A future protocol version should replace this with
/// `interop_hash` for consistency. This changes all method IDs and requires
/// a coordinated native contract + host upgrade.
fn native_method_id(method: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for byte in method.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

/// Execute a NeoVM script with a caller-supplied syscall callback and an empty
/// initial stack.
///
/// Shorthand for [`execute_script_with_host_and_stack`] with an empty initial
/// stack — the common case when the script builds its own stack from scratch.
pub fn execute_script_with_host<F>(
    script: &[u8],
    context: RuntimeContext,
    callback: F,
) -> Result<ExecutionResult, String>
where
    F: FnMut(
        u32,
        usize,
        RuntimeContext,
        &[neo_riscv_abi::StackValue],
    ) -> Result<HostCallbackResult, String>,
{
    execute_script_with_host_and_stack(script, Vec::new(), context, callback)
}

/// Execute a native RISC-V contract binary's method by name, using the built-in
/// storage/syscall backend.
///
/// Resolves `method` to a stable id via [`native_method_id`], then delegates to
/// [`execute_native_contract_builtin_by_id`]. Syscalls and storage are served by
/// the in-process `BuiltinStorage` cache (no host callback required).
pub fn execute_native_contract_builtin(
    binary: &[u8],
    method: &str,
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    context: RuntimeContext,
) -> Result<ExecutionResult, String> {
    execute_native_contract_builtin_by_id(binary, native_method_id(method), initial_stack, context)
}

/// Execute a native RISC-V contract binary's method by pre-computed method id,
/// using the built-in storage/syscall backend.
///
/// Like [`execute_native_contract_builtin`] but takes the method id directly,
/// avoiding the per-call FNV-1a hash. Used in hot paths where the id is already
/// known.
pub fn execute_native_contract_builtin_by_id(
    binary: &[u8],
    method_id: u32,
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    context: RuntimeContext,
) -> Result<ExecutionResult, String> {
    reset_execution_sidechannels();
    let stack_bytes = fast_codec::encode_stack(&initial_stack);
    let max_stack_len = stack_bytes.len();
    let aux_size = if max_stack_len == 0 {
        0
    } else {
        align_up_u32(
            u32::try_from(max_stack_len)
                .map_err(|_| "native contract stack too large".to_string())?,
            8,
        )
    };

    let mut cached_instance = runtime_cache::cached_native_execution_instance(binary, aux_size)?;
    let aux_base = if !stack_bytes.is_empty() {
        cached_instance.module().memory_map().aux_data_address()
    } else {
        0
    };
    let instance = cached_instance.instance_mut();
    let mut host = bridge::ClosureHost::new_builtin(context);
    let native_gas_limit = crate::pricing::native_instruction_limit(&host.context);

    let stack_ptr = if !stack_bytes.is_empty() {
        if aux_size > 0 {
            instance
                .set_accessible_aux_size(aux_size)
                .map_err(|e| format!("native aux setup failed: {e}"))?;
        }
        instance
            .write_memory(aux_base, &stack_bytes)
            .map_err(|e| format!("native write_memory failed: {e:?}"))?;
        aux_base
    } else {
        0
    };
    let stack_len: u32 = stack_bytes
        .len()
        .try_into()
        .map_err(|_| "native contract stack too large for u32".to_string())?;

    instance.set_gas(native_gas_limit);
    let call_result = instance.call_typed(
        &mut host,
        "execute_method",
        (method_id, stack_ptr, stack_len),
    );

    if let Err(error) = call_result {
        let charge_error =
            charge_native_metered_instructions(instance, &mut host, native_gas_limit).err();
        if let Some(message) = charge_error
            .as_deref()
            .or_else(|| native_call_error(&error).or(host.charge_error.as_deref()))
        {
            return Err(message.to_string());
        }
        instance.set_gas(crate::pricing::NEO_INSTRUCTION_CEILING as i64);
        let trace = read_guest_trace(instance, &mut host);
        let panic_msg = read_guest_panic(instance, &mut host);
        let debug = read_guest_debug(instance, &mut host);
        let alloc_peak = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ())
            .unwrap_or(0);
        let alloc_fails = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ())
            .unwrap_or(0);
        let alloc_fail_size = instance
            .call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ())
            .unwrap_or(0);
        return Err(format!(
            "native contract execute failed: {error:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}; panic={panic_msg:?}; debug={debug:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
            host.last_opcode,
            host.opcode_count,
            host.syscall_count,
            host.last_api,
            host.last_ip,
            host.last_stack_len,
            host.last_result_cap,
            host.last_host_call_stage
        ));
    }

    let res_ptr: u32 =
        match instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ()) {
            Ok(value) => value,
            Err(error) => {
                charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;
                if let Some(message) = native_call_error(&error) {
                    return Err(message.to_string());
                }
                return Err(format!("native get_result_ptr failed: {error:?}"));
            }
        };
    let res_len: u32 =
        match instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ()) {
            Ok(value) => value,
            Err(error) => {
                charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;
                if let Some(message) = native_call_error(&error) {
                    return Err(message.to_string());
                }
                return Err(format!("native get_result_len failed: {error:?}"));
            }
        };

    if res_len > MAX_RESULT_SIZE {
        return Err(format!(
            "native result size {res_len} exceeds maximum {MAX_RESULT_SIZE}"
        ));
    }
    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("native read_memory failed: {e:?}"))?;

    charge_native_metered_instructions(instance, &mut host, native_gas_limit)?;

    let mut result = neo_riscv_abi::result_codec::decode_execution_result(&res_bytes)
        .map_err(|_| "failed to decode native result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    if let Ok(ref r) = result
        && r.state == VmState::Fault
    {
        // Capture IP and locals in thread-local side-channels before the Ok(Fault)→Err
        // conversion loses them. C# retrieves via `neo_riscv_last_fault_ip()` and
        // `neo_riscv_last_fault_locals()`.
        set_last_fault_ip(r.fault_ip);
        set_last_fault_locals(&r.fault_locals);
        // Prefer host.charge_error when the host-side charge failed (gas exhaustion or
        // instruction ceiling): the guest only saw the import's 0-return and replied
        // with a generic "host instruction charge failed". host.charge_error preserves
        // the specific reason.
        if let Some(ref msg) = host.charge_error {
            return Err(msg.clone());
        }
        if let Some(ref msg) = r.fault_message {
            return Err(msg.clone());
        }
    }

    result
}

#[allow(clippy::if_same_then_else)]
pub(crate) fn builtin_host_callback(
    api: u32,
    context: RuntimeContext,
    _stack: &[neo_riscv_abi::StackValue],
) -> Result<HostCallbackResult, String> {
    use neo_riscv_abi::StackValue;
    use neo_riscv_abi::interop_hash;

    // All builtin syscalls are 0-arg: they receive an empty stack and return [result].
    // The caller (invoke_syscall) handles popping consumed args and pushing results.

    // System.Runtime (zero-arg getters)
    if api == interop_hash("System.Runtime.Platform") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(b"NEO".to_vec())],
        })
    } else if api == interop_hash("System.Runtime.GetTrigger") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.trigger))],
        })
    } else if api == interop_hash("System.Runtime.GetNetwork") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.network))],
        })
    } else if api == interop_hash("System.Runtime.GetAddressVersion") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(i64::from(context.address_version))],
        })
    } else if api == interop_hash("System.Runtime.GasLeft") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(context.gas_left)],
        })
    } else if api == interop_hash("System.Runtime.GetTime") {
        match context.timestamp {
            Some(timestamp) => Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(timestamp as i64)],
            }),
            None => Err("GetTime requires a persisting block timestamp".to_string()),
        }
    } else if api == interop_hash("System.Runtime.GetScriptContainer") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Runtime.GetExecutingScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetCallingScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetEntryScriptHash") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Runtime.GetInvocationCounter") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Runtime.GetRandom") {
        // Simple deterministic random for standalone execution
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(42)],
        })
    } else if api == interop_hash("System.Runtime.CurrentSigners") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Runtime.GetNotifications") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Array(Vec::new())],
        })
    } else if api == interop_hash("System.Runtime.CheckWitness") {
        // In standalone mode, always return true
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    } else if api == interop_hash("System.Runtime.BurnGas") {
        // In standalone mode, gas burning is a no-op
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.Notify") {
        // In standalone mode, notify is a no-op
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.Log") {
        // In standalone mode, log is a no-op
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Runtime.LoadScript") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    // System.Storage
    } else if api == interop_hash("System.Storage.GetContext") {
        // Return a dummy context handle
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.GetReadOnlyContext") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.AsReadOnly") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(1)],
        })
    } else if api == interop_hash("System.Storage.Get") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Put") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Delete") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Find") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Local.Get") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Storage.Local.Put") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Local.Delete") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Storage.Local.Find") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    // System.Contract
    } else if api == interop_hash("System.Contract.Call") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Contract.Create") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else if api == interop_hash("System.Contract.Update") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Contract.GetCallFlags") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Integer(0x0f)],
        })
    } else if api == interop_hash("System.Contract.CreateStandardAccount") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Contract.CreateMultisigAccount") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::ByteString(vec![0u8; 20])],
        })
    } else if api == interop_hash("System.Contract.NativeOnPersist") {
        Ok(HostCallbackResult { stack: vec![] })
    } else if api == interop_hash("System.Contract.NativePostPersist") {
        Ok(HostCallbackResult { stack: vec![] })
    // System.Crypto
    } else if api == interop_hash("System.Crypto.CheckSig") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    } else if api == interop_hash("System.Crypto.CheckMultisig") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(true)],
        })
    // System.Iterator
    } else if api == interop_hash("System.Iterator.Next") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Boolean(false)],
        })
    } else if api == interop_hash("System.Iterator.Value") {
        Ok(HostCallbackResult {
            stack: vec![StackValue::Null],
        })
    } else {
        Err(format!("unsupported syscall 0x{api:08x}"))
    }
}
