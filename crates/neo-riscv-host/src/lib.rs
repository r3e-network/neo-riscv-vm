mod bridge;
mod ffi;
mod pricing;
mod runtime_cache;

use bridge::{read_guest_trace, ClosureHost, GuestTrace};
use neo_riscv_abi::{BackendKind, ExecutionResult, VmState};

pub use ffi::{
    neo_riscv_execute_native_contract, neo_riscv_execute_script,
    neo_riscv_execute_script_with_host, neo_riscv_free_execution_result, NativeExecutionResult,
    NativeHostCallback, NativeHostFreeCallback, NativeHostResult, NativeStackItem,
};

pub struct PolkaVmRuntime {
    backend_kind: BackendKind,
}

#[derive(Clone, Copy)]
pub struct RuntimeContext {
    pub trigger: u8,
    pub network: u32,
    pub address_version: u8,
    pub timestamp: Option<u64>,
    pub gas_left: i64,
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
    let aux_size = required_aux_size(script.len() as u32, initial_stack_bytes_len(&initial_stack));
    let mut cached_instance = runtime_cache::cached_execution_instance(aux_size)?;
    let mut host = ClosureHost::new(context, &mut callback);
    let aux_base = cached_instance.module().memory_map().aux_data_address();
    let instance = cached_instance.instance_mut();

    let script_len = script.len() as u32;
    let initial_stack_bytes = postcard::to_allocvec(&initial_stack)
        .map_err(|e| format!("failed to serialize initial stack: {e}"))?;
    let stack_len = initial_stack_bytes.len() as u32;
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
            format!(
                "guest execute failed: {e:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}",
                host.last_opcode,
                host.opcode_count,
                host.syscall_count,
                host.last_api,
                host.last_ip,
                host.last_stack_len,
                host.last_result_cap,
                host.last_host_call_stage
            )
        })?;

    let trace = read_guest_trace(instance, &mut host);

    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("guest get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("guest get_result_len failed: {e:?}"))?;

    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("guest read_memory failed: {e:?}"))?;
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
    let aux_size = required_aux_size(script.len() as u32, initial_stack_bytes_len(&initial_stack));
    let mut cached_instance = runtime_cache::cached_execution_instance(aux_size)?;
    let mut host = ClosureHost::new(context, &mut callback);
    let aux_base = cached_instance.module().memory_map().aux_data_address();
    let instance = cached_instance.instance_mut();

    let script_len = script.len() as u32;
    let initial_stack_bytes = postcard::to_allocvec(&initial_stack)
        .map_err(|e| format!("failed to serialize initial stack: {e}"))?;
    let stack_len = initial_stack_bytes.len() as u32;
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
        let verify = instance
            .read_memory(script_ptr, script_len)
            .map_err(|e| format!("guest readback failed: {e:?}"))?;
        if verify != script {
            return Err("guest aux script readback mismatch".to_string());
        }
    }

    if stack_len > 0 {
        instance
            .write_memory(stack_ptr, &initial_stack_bytes)
            .map_err(|e| format!("guest write_memory failed: {e:?}"))?;
        let verify = instance
            .read_memory(stack_ptr, stack_len)
            .map_err(|e| format!("guest stack readback failed: {e:?}"))?;
        if verify != initial_stack_bytes {
            return Err("guest aux stack readback mismatch".to_string());
        }
    }

    instance
        .call_typed(
            &mut host,
            "execute",
            (script_ptr, script_len, stack_ptr, stack_len, initial_ip as u32),
        )
        .map_err(|e| {
            let trace = read_guest_trace(instance, &mut host);
            format!(
                "guest execute failed: {e:?}; last_opcode={:?}; opcode_count={}; syscall_count={}; last_api={:?}; last_ip={:?}; last_stack_len={:?}; last_result_cap={:?}; last_host_call_stage={}; trace={trace:?}",
                host.last_opcode,
                host.opcode_count,
                host.syscall_count,
                host.last_api,
                host.last_ip,
                host.last_stack_len,
                host.last_result_cap,
                host.last_host_call_stage
            )
        })?;

    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("guest get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("guest get_result_len failed: {e:?}"))?;

    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("guest read_memory failed: {e:?}"))?;
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

fn initial_stack_bytes_len(initial_stack: &[neo_riscv_abi::StackValue]) -> u32 {
    postcard::to_allocvec(initial_stack)
        .map(|bytes| bytes.len() as u32)
        .unwrap_or(0)
}

fn align_up_u32(value: u32, align: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.div_ceil(align) * align
    }
}

fn required_aux_size(script_len: u32, stack_len: u32) -> u32 {
    if script_len == 0 && stack_len == 0 {
        0
    } else {
        align_up_u32(script_len, 8).saturating_add(stack_len)
    }
}

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

    let stack_bytes = postcard::to_allocvec(&full_stack)
        .map_err(|e| format!("failed to serialize native contract stack: {e}"))?;
    let aux_size = if stack_bytes.is_empty() {
        0
    } else {
        align_up_u32(stack_bytes.len() as u32, 8)
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
    let stack_len = stack_bytes.len() as u32;

    // Call the contract's execute entry point
    instance
        .call_typed(&mut host, "execute", (stack_ptr, stack_len))
        .map_err(|e| format!("native contract execute failed: {e:?}"))?;

    // Read result back
    let res_ptr: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_ptr", ())
        .map_err(|e| format!("native get_result_ptr failed: {e:?}"))?;
    let res_len: u32 = instance
        .call_typed_and_get_result::<u32, ()>(&mut host, "get_result_len", ())
        .map_err(|e| format!("native get_result_len failed: {e:?}"))?;

    let mut res_bytes = vec![0u8; res_len as usize];
    instance
        .read_memory_into(res_ptr, &mut res_bytes[..])
        .map_err(|e| format!("native read_memory failed: {e:?}"))?;

    let mut result: Result<ExecutionResult, String> = postcard::from_bytes(&res_bytes)
        .map_err(|_| "failed to decode native result".to_string())?;

    if let Ok(ref mut r) = result {
        r.fee_consumed_pico = host.fee_consumed_pico;
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
    stack: &[neo_riscv_abi::StackValue],
) -> Result<HostCallbackResult, String> {
    let mut next_stack = stack.to_vec();
    if api == neo_riscv_abi::interop_hash("System.Runtime.Platform") {
        next_stack.push(neo_riscv_abi::StackValue::ByteString(b"NEO".to_vec()));
        Ok(HostCallbackResult { stack: next_stack })
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.GetTrigger") {
        next_stack.push(neo_riscv_abi::StackValue::Integer(i64::from(
            context.trigger,
        )));
        Ok(HostCallbackResult { stack: next_stack })
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.GetNetwork") {
        next_stack.push(neo_riscv_abi::StackValue::Integer(i64::from(
            context.network,
        )));
        Ok(HostCallbackResult { stack: next_stack })
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.GetAddressVersion") {
        next_stack.push(neo_riscv_abi::StackValue::Integer(i64::from(
            context.address_version,
        )));
        Ok(HostCallbackResult { stack: next_stack })
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.GasLeft") {
        next_stack.push(neo_riscv_abi::StackValue::Integer(context.gas_left));
        Ok(HostCallbackResult { stack: next_stack })
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.GetTime") {
        match context.timestamp {
            Some(timestamp) => {
                next_stack.push(neo_riscv_abi::StackValue::Integer(timestamp as i64));
                Ok(HostCallbackResult { stack: next_stack })
            }
            None => Err("GetTime requires a persisting block timestamp".to_string()),
        }
    } else {
        Err(format!("unsupported syscall 0x{api:08x}"))
    }
}
