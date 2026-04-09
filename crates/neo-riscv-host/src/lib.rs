//! Neo RISC-V host runtime.
//!
//! Provides the host-side VM runtime using PolkaVM, FFI bindings for C# interop,
//! and execution context management.

mod bridge;
mod ffi;
mod pricing;
mod profiling;
mod runtime_cache;

use bridge::{
    read_guest_debug, read_guest_panic, read_guest_trace, read_pc_trace, ClosureHost, GuestTrace,
};
use neo_riscv_abi::{fast_codec, BackendKind, ExecutionResult, VmState};

/// Maximum allowed result size from guest (16 MB).
const MAX_RESULT_SIZE: u32 = 16 * 1024 * 1024;

pub use ffi::{
    neo_riscv_execute_native_contract, neo_riscv_execute_script,
    neo_riscv_execute_script_with_host, neo_riscv_free_execution_result, NativeExecutionResult,
    NativeHostCallback, NativeHostFreeCallback, NativeHostResult, NativeStackItem,
};
pub use profiling::{get_current_memory, get_peak_memory, reset as reset_profiling};

/// PolkaVM runtime instance.
pub struct PolkaVmRuntime {
    backend_kind: BackendKind,
}

/// Runtime execution context for VM scripts.
#[derive(Clone, Copy)]
pub struct RuntimeContext {
    /// Trigger type (Application, Verification, etc.).
    pub trigger: u8,
    /// Network magic number.
    pub network: u32,
    /// Address version byte.
    pub address_version: u8,
    /// Block timestamp (optional).
    pub timestamp: Option<u64>,
    /// Remaining gas.
    pub gas_left: i64,
    /// Gas price factor in pico units.
    pub exec_fee_factor_pico: i64,
}

impl PolkaVmRuntime {
    pub fn new() -> Result<Self, String> {
        runtime_cache::ensure_runtime_ready()?;

        Ok(Self {
            backend_kind: BackendKind::Interpreter,
        })
    }

    #[must_use]
    pub fn backend_kind(&self) -> BackendKind {
        self.backend_kind
    }
}

pub fn execute_script(script: &[u8]) -> Result<ExecutionResult, String> {
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

pub fn execute_script_with_trigger(script: &[u8], trigger: u8) -> Result<ExecutionResult, String> {
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

pub fn execute_script_with_context(
    script: &[u8],
    context: RuntimeContext,
) -> Result<ExecutionResult, String> {
    execute_script_with_host_and_stack_and_ip(
        script,
        Vec::new(),
        0,
        context,
        |api, _ip, context, stack| builtin_host_callback(api, context, stack),
    )
}

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
            let trace = read_guest_trace(instance, &mut host);
            // Read guest panic message if available
            let panic_msg = read_guest_panic(instance, &mut host);
            {
                let alloc_peak = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ()).unwrap_or(0);
                let alloc_fails = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ()).unwrap_or(0);
                let alloc_fail_size = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ()).unwrap_or(0);
                format!(
                    "guest execute failed: {e:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}; panic={panic_msg:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
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
    // Debug: uncomment to trace RESULT_BYTES
    // println!("Guest RESULT_BYTES ({} bytes): {:?}", res_len, res_bytes);
    let mut result: Result<ExecutionResult, String> =
        postcard::from_bytes(&res_bytes).map_err(|_| "Failed to decode result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the guest returned a VM-level FAULT with fault_message, return it without
    // internal trace (trace is for FFI-level errors only).
    if let Ok(ref r) = result {
        if r.state == VmState::Fault {
            if let Some(ref msg) = r.fault_message {
                return Err(msg.clone());
            }
        }
    }

    match result {
        Ok(result) => Ok((result, trace)),
        Err(error) => Err(format!("{error}; trace={trace:?}")),
    }
}

pub fn execute_script_with_host_and_stack_and_ip<F>(
    script: &[u8],
    initial_stack: Vec<neo_riscv_abi::StackValue>,
    initial_ip: usize,
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
            (script_ptr, script_len, stack_ptr, stack_len, initial_ip as u32),
        )
        .map_err(|e| {
            let trace = read_guest_trace(instance, &mut host);
            let panic_msg = read_guest_panic(instance, &mut host);
            {
                let alloc_peak = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_peak", ()).unwrap_or(0);
                let alloc_fails = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_count", ()).unwrap_or(0);
                let alloc_fail_size = instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_allocator_fail_size", ()).unwrap_or(0);
                format!(
                    "guest execute failed: {e:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}; panic={panic_msg:?}; alloc_peak={alloc_peak}; alloc_fails={alloc_fails}; alloc_fail_size={alloc_fail_size}",
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
    // Debug: uncomment to trace RESULT_BYTES
    // println!("Guest RESULT_BYTES ({} bytes): {:?}", res_len, res_bytes);
    let mut result: Result<ExecutionResult, String> =
        postcard::from_bytes(&res_bytes).map_err(|_| "Failed to decode result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the guest returned a VM-level FAULT with fault_message, return it without
    // internal trace (trace is for FFI-level errors only).
    if let Ok(ref r) = result {
        if r.state == VmState::Fault {
            if let Some(ref msg) = r.fault_message {
                return Err(msg.clone());
            }
        }
    }

    if let Err(error) = result {
        let trace = read_guest_trace(instance, &mut host);
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

#[derive(Debug)]
pub struct HostCallbackResult {
    pub stack: Vec<neo_riscv_abi::StackValue>,
}

/// Execute a native RISC-V contract binary directly via PolkaVM.
///
/// Unlike `execute_script_*` which runs NeoVM bytecode through the cached
/// interpreter guest, this compiles the contract binary itself as a PolkaVM
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
    // Prepend method name to stack so the contract can dispatch
    let mut full_stack = Vec::with_capacity(1 + initial_stack.len());
    full_stack.push(neo_riscv_abi::StackValue::ByteString(
        method.as_bytes().to_vec(),
    ));
    full_stack.extend(initial_stack);

    let stack_bytes = fast_codec::encode_stack(&full_stack);

    // DEBUG: log encoded stack bytes for native contract execution
    eprintln!(
        "[NATIVE_DEBUG] method={} encoded_stack ({} bytes): {:?}",
        method,
        stack_bytes.len(),
        &stack_bytes[..std::cmp::min(128, stack_bytes.len())]
    );
    let aux_size = if stack_bytes.is_empty() {
        0
    } else {
        align_up_u32(
            u32::try_from(stack_bytes.len())
                .map_err(|_| "native contract stack too large".to_string())?,
            8,
        )
    };

    // Compile the contract binary as a fresh PolkaVM module
    let module = runtime_cache::compile_native_module(binary, aux_size)?;

    // Link with the same host functions (host_call, host_on_instruction)
    let mut linker = polkavm::Linker::<bridge::ClosureHost, core::convert::Infallible>::new();
    bridge::register_host_functions(&mut linker)?;
    let instance_pre = linker.instantiate_pre(&module).map_err(|e| e.to_string())?;
    let mut instance = instance_pre.instantiate().map_err(|e| e.to_string())?;

    let mut host = bridge::ClosureHost::new(context, &mut callback);

    // Write serialized stack to aux memory
    let stack_ptr = if !stack_bytes.is_empty() {
        let aux_base = module.memory_map().aux_data_address();
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

    // Call the contract's execute entry point
    let call_result = instance.call_typed(&mut host, "execute", (stack_ptr, stack_len));

    // DEBUG: log execution diagnostics
    {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/neo-riscv-bridge-debug.log")
        {
            let _ = writeln!(f, "[HOST] execute returned: {:?}", call_result.is_ok());
            let _ = writeln!(
                f,
                "[HOST] opcode_count={} syscall_count={} last_api=0x{:08x} last_host_call_stage={}",
                host.opcode_count,
                host.syscall_count,
                host.last_api.unwrap_or(0),
                host.last_host_call_stage
            );
            let _ = writeln!(
                f,
                "[HOST] fee_consumed_pico={} gas_left={}",
                host.fee_consumed_pico, host.context.gas_left
            );
            // Read guest-side debug buffer
            if let Some(debug_bytes) = read_guest_debug(&mut instance, &mut host) {
                let _ = writeln!(f, "[HOST] guest_debug: {} bytes", debug_bytes.len());
                // Parse debug records: each is 23 bytes + variable fault msg
                let mut offset = 0;
                while offset + 23 <= debug_bytes.len() {
                    let step = debug_bytes[offset];
                    let api =
                        u32::from_le_bytes(debug_bytes[offset + 1..offset + 5].try_into().unwrap());
                    let stack_len =
                        u32::from_le_bytes(debug_bytes[offset + 5..offset + 9].try_into().unwrap());
                    let arg_count = u32::from_le_bytes(
                        debug_bytes[offset + 9..offset + 13].try_into().unwrap(),
                    );
                    let encoded_len = u32::from_le_bytes(
                        debug_bytes[offset + 13..offset + 17].try_into().unwrap(),
                    );
                    let result_len = u32::from_le_bytes(
                        debug_bytes[offset + 17..offset + 21].try_into().unwrap(),
                    );
                    let fault_len = u16::from_le_bytes(
                        debug_bytes[offset + 21..offset + 23].try_into().unwrap(),
                    );
                    let fault =
                        if fault_len > 0 && offset + 23 + fault_len as usize <= debug_bytes.len() {
                            String::from_utf8_lossy(
                                &debug_bytes[offset + 23..offset + 23 + fault_len as usize],
                            )
                            .to_string()
                        } else {
                            String::new()
                        };
                    let _ = writeln!(f, "[GUEST] step={} api=0x{:08x} stack_len={} arg_count={} encoded_len={} result_len={} fault={:?}",
                        step, api, stack_len, arg_count, encoded_len, result_len, fault);
                    offset += 23 + fault_len as usize;
                }
            }
            // Read PC trace
            if let Some(pc_bytes) = read_pc_trace(&mut instance, &mut host) {
                let _ = writeln!(f, "[HOST] pc_trace: {:?}", pc_bytes);
            }
            // Read diagnostic buffer
            let diag_ptr_res =
                instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_diag_ptr", ());
            let diag_len_res =
                instance.call_typed_and_get_result::<u32, ()>(&mut host, "get_diag_len", ());
            let _ = writeln!(
                f,
                "[HOST] diag_ptr_res={:?} diag_len_res={:?}",
                diag_ptr_res, diag_len_res
            );
            if let (Ok(diag_ptr), Ok(diag_len)) = (diag_ptr_res, diag_len_res) {
                if diag_len > 0 {
                    let mut diag_bytes = vec![0u8; diag_len as usize];
                    if instance
                        .read_memory_into(diag_ptr, &mut diag_bytes[..])
                        .is_ok()
                    {
                        let _ = writeln!(f, "[HOST] diag: {} bytes", diag_bytes.len());
                        // Parse: markers 0xA0=args_after_init, 0xA1=args_before_put, 0xA2=locals, 0xA3=stack_before_put
                        let mut i = 0;
                        while i < diag_bytes.len() {
                            let marker = diag_bytes[i];
                            i += 1;
                            if marker == 0xA0 || marker == 0xA1 {
                                if i + 4 > diag_bytes.len() {
                                    break;
                                }
                                let count =
                                    u32::from_le_bytes(diag_bytes[i..i + 4].try_into().unwrap());
                                i += 4;
                                let label = if marker == 0xA0 {
                                    "args_init"
                                } else {
                                    "args_pre_put"
                                };
                                let _ = write!(f, "[HOST] diag {label}({count}): ");
                                for _ in 0..count {
                                    if i >= diag_bytes.len() {
                                        break;
                                    }
                                    let typ = diag_bytes[i];
                                    i += 1;
                                    match typ {
                                        1 => {
                                            // Integer
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let v = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let _ = write!(f, "Int({v}) ");
                                        }
                                        3 => {
                                            // ByteString
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let len = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let data_end = (i + len as usize).min(diag_bytes.len());
                                            let data = &diag_bytes[i..data_end];
                                            i += len as usize;
                                            let _ = write!(
                                                f,
                                                "Bytes({len}:{:?}) ",
                                                core::str::from_utf8(data).unwrap_or("<bin>")
                                            );
                                        }
                                        4 => {
                                            // Boolean
                                            if i >= diag_bytes.len() {
                                                break;
                                            }
                                            let v = diag_bytes[i];
                                            i += 1;
                                            let _ = write!(f, "Bool({v}) ");
                                        }
                                        _ => {
                                            let _ = write!(f, "Unknown({typ}) ");
                                        }
                                    }
                                }
                                let _ = writeln!(f, "");
                            } else if marker == 0xA2 {
                                if i + 4 > diag_bytes.len() {
                                    break;
                                }
                                let count =
                                    u32::from_le_bytes(diag_bytes[i..i + 4].try_into().unwrap());
                                i += 4;
                                let _ = write!(f, "[HOST] diag locals({count}): ");
                                for _ in 0..count {
                                    if i >= diag_bytes.len() {
                                        break;
                                    }
                                    let typ = diag_bytes[i];
                                    i += 1;
                                    match typ {
                                        1 => {
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let v = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let _ = write!(f, "Int({v}) ");
                                        }
                                        3 => {
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let len = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let data_end = (i + len as usize).min(diag_bytes.len());
                                            let data = &diag_bytes[i..data_end];
                                            i += len as usize;
                                            let _ = write!(
                                                f,
                                                "Bytes({len}:{:?}) ",
                                                core::str::from_utf8(data).unwrap_or("<bin>")
                                            );
                                        }
                                        4 => {
                                            if i >= diag_bytes.len() {
                                                break;
                                            }
                                            let v = diag_bytes[i];
                                            i += 1;
                                            let _ = write!(f, "Bool({v}) ");
                                        }
                                        _ => {
                                            let _ = write!(f, "Unknown({typ}) ");
                                        }
                                    }
                                }
                                let _ = writeln!(f, "");
                            } else if marker == 0xA3 {
                                if i + 4 > diag_bytes.len() {
                                    break;
                                }
                                let count =
                                    u32::from_le_bytes(diag_bytes[i..i + 4].try_into().unwrap());
                                i += 4;
                                let _ = write!(f, "[HOST] diag stack_pre_put({count}): ");
                                for _ in 0..count {
                                    if i >= diag_bytes.len() {
                                        break;
                                    }
                                    let typ = diag_bytes[i];
                                    i += 1;
                                    match typ {
                                        1 => {
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let v = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let _ = write!(f, "Int({v}) ");
                                        }
                                        3 => {
                                            if i + 4 > diag_bytes.len() {
                                                break;
                                            }
                                            let len = u32::from_le_bytes(
                                                diag_bytes[i..i + 4].try_into().unwrap(),
                                            );
                                            i += 4;
                                            let data_end = (i + len as usize).min(diag_bytes.len());
                                            let data = &diag_bytes[i..data_end];
                                            i += len as usize;
                                            let _ = write!(
                                                f,
                                                "Bytes({len}:{:?}) ",
                                                core::str::from_utf8(data).unwrap_or("<bin>")
                                            );
                                        }
                                        4 => {
                                            if i >= diag_bytes.len() {
                                                break;
                                            }
                                            let v = diag_bytes[i];
                                            i += 1;
                                            let _ = write!(f, "Bool({v}) ");
                                        }
                                        _ => {
                                            let _ = write!(f, "Unknown({typ}) ");
                                        }
                                    }
                                }
                                let _ = writeln!(f, "");
                            } else {
                                let _ = writeln!(f, "[HOST] diag unknown marker 0x{marker:02x}");
                            }
                        }
                    }
                }
            }
        }
    }

    call_result.map_err(|e| format!("native contract execute failed: {e:?}"))?;

    // Read result back
    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("native get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("native get_result_len failed: {e:?}"))?;

    if res_len > MAX_RESULT_SIZE {
        return Err(format!(
            "native result size {res_len} exceeds maximum {MAX_RESULT_SIZE}"
        ));
    }
    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("native read_memory failed: {e:?}"))?;

    // Debug: uncomment to trace RESULT_BYTES
    // println!("Guest RESULT_BYTES ({} bytes): {:?}", res_len, res_bytes);
    let mut result: Result<ExecutionResult, String> = postcard::from_bytes(&res_bytes)
        .map_err(|_| "failed to decode native result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
    }

    // If the contract returned a VM-level FAULT with fault_message, return as error
    // (consistent with execute_script_* paths).
    if let Ok(ref r) = result {
        if r.state == VmState::Fault {
            if let Some(ref msg) = r.fault_message {
                return Err(msg.clone());
            }
        }
    }

    result
}

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

fn builtin_host_callback(
    api: u32,
    context: RuntimeContext,
    _stack: &[neo_riscv_abi::StackValue],
) -> Result<HostCallbackResult, String> {
    use neo_riscv_abi::interop_hash;
    use neo_riscv_abi::StackValue;

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
