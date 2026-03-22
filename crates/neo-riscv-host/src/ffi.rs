use crate::pricing::charge_opcode;
use crate::{
    execute_script_with_context, execute_script_with_host_and_stack_and_ip, HostCallbackResult,
    RuntimeContext,
};
use neo_riscv_abi::ExecutionResult;
use neo_riscv_guest::SyscallProvider;
use std::{ffi::c_void, ptr, slice};

#[repr(C)]
pub struct NativeHostResult {
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *mut u8,
    pub error_len: usize,
}

pub type NativeHostCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    api: u32,
    instruction_pointer: usize,
    trigger: u8,
    network: u32,
    address_version: u8,
    timestamp: u64,
    gas_left: i64,
    input_stack_ptr: *const NativeStackItem,
    input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool;

pub type NativeHostFreeCallback =
    unsafe extern "C" fn(user_data: *mut c_void, result: *mut NativeHostResult);

struct FfiHost {
    context: RuntimeContext,
    fee_consumed_pico: i64,
    user_data: *mut c_void,
    callback: NativeHostCallback,
    free_callback: NativeHostFreeCallback,
}

impl SyscallProvider for FfiHost {
    fn on_instruction(&mut self, opcode: u8) -> Result<(), String> {
        charge_opcode(&mut self.context, &mut self.fee_consumed_pico, opcode)
    }

    fn syscall(
        &mut self,
        api: u32,
        ip: usize,
        stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        let mut result = NativeHostResult {
            stack_ptr: ptr::null_mut(),
            stack_len: 0,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
        let (input_stack_ptr, input_stack_len) = serialize_stack_items(stack);

        let invoked = unsafe {
            (self.callback)(
                self.user_data,
                api,
                ip,
                self.context.trigger,
                self.context.network,
                self.context.address_version,
                self.context.timestamp.unwrap_or_default(),
                self.context.gas_left,
                input_stack_ptr,
                input_stack_len,
                &mut result,
            )
        };
        free_native_stack_items(input_stack_ptr, input_stack_len);

        if !invoked {
            return Err(format!(
                "host callback invocation failed for syscall 0x{api:08x}"
            ));
        }

        let host_result = copy_native_host_result(&result);
        unsafe {
            (self.free_callback)(self.user_data, &mut result);
        }

        let host_result = host_result?;
        *stack = host_result.stack;
        Ok(())
    }
}

#[repr(C)]
pub struct NativeExecutionResult {
    pub fee_consumed_pico: i64,
    pub state: u32,
    pub stack_ptr: *mut NativeStackItem,
    pub stack_len: usize,
    pub error_ptr: *mut u8,
    pub error_len: usize,
}

#[repr(C)]
pub struct NativeStackItem {
    pub kind: u32,
    pub integer_value: i64,
    pub bytes_ptr: *mut u8,
    pub bytes_len: usize,
}

fn copy_native_stack_items(
    stack_ptr: *mut NativeStackItem,
    stack_len: usize,
) -> Result<Vec<neo_riscv_abi::StackValue>, String> {
    let mut stack = Vec::with_capacity(stack_len);

    for index in 0..stack_len {
        let item_ptr = unsafe { stack_ptr.add(index) };
        let item = unsafe { &*item_ptr };
        match item.kind {
            0 => stack.push(neo_riscv_abi::StackValue::Integer(item.integer_value)),
            5 => {
                let bytes = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe { slice::from_raw_parts(item.bytes_ptr, item.bytes_len) }.to_vec()
                };
                stack.push(neo_riscv_abi::StackValue::BigInteger(bytes));
            }
            9 => stack.push(neo_riscv_abi::StackValue::Interop(
                item.integer_value as u64,
            )),
            6 => stack.push(neo_riscv_abi::StackValue::Iterator(
                item.integer_value as u64,
            )),
            1 => {
                let bytes = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe { slice::from_raw_parts(item.bytes_ptr, item.bytes_len) }.to_vec()
                };
                stack.push(neo_riscv_abi::StackValue::ByteString(bytes));
            }
            3 => stack.push(neo_riscv_abi::StackValue::Boolean(item.integer_value != 0)),
            4 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    copy_native_stack_items(
                        item.bytes_ptr.cast::<NativeStackItem>(),
                        item.bytes_len,
                    )?
                };
                stack.push(neo_riscv_abi::StackValue::Array(items));
            }
            7 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    copy_native_stack_items(
                        item.bytes_ptr.cast::<NativeStackItem>(),
                        item.bytes_len,
                    )?
                };
                stack.push(neo_riscv_abi::StackValue::Struct(items));
            }
            8 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    copy_native_stack_items(
                        item.bytes_ptr.cast::<NativeStackItem>(),
                        item.bytes_len,
                    )?
                };
                if items.len() % 2 != 0 {
                    return Err("map stack item contains an odd number of entries".to_string());
                }
                let mut pairs = Vec::with_capacity(items.len() / 2);
                let mut iter = items.into_iter();
                while let Some(key) = iter.next() {
                    let value = iter.next().ok_or_else(|| {
                        "map stack item contains an incomplete key/value pair".to_string()
                    })?;
                    pairs.push((key, value));
                }
                stack.push(neo_riscv_abi::StackValue::Map(pairs));
            }
            2 => stack.push(neo_riscv_abi::StackValue::Null),
            10 => stack.push(neo_riscv_abi::StackValue::Pointer(item.integer_value)),
            other => return Err(format!("unsupported native stack item kind {other}")),
        }
    }

    Ok(stack)
}

fn serialize_stack_items(stack: &[neo_riscv_abi::StackValue]) -> (*mut NativeStackItem, usize) {
    if stack.is_empty() {
        return (ptr::null_mut(), 0);
    }

    let native_stack = stack
        .iter()
        .map(|value| match value {
            neo_riscv_abi::StackValue::Integer(value) => NativeStackItem {
                kind: 0,
                integer_value: *value,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            neo_riscv_abi::StackValue::BigInteger(value) => {
                let bytes = value.clone().into_boxed_slice();
                let bytes_len = bytes.len();
                let bytes_ptr = Box::into_raw(bytes) as *mut u8;
                NativeStackItem {
                    kind: 5,
                    integer_value: 0,
                    bytes_ptr,
                    bytes_len,
                }
            }
            neo_riscv_abi::StackValue::Iterator(handle) => NativeStackItem {
                kind: 6,
                integer_value: *handle as i64,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            neo_riscv_abi::StackValue::Interop(handle) => NativeStackItem {
                kind: 9,
                integer_value: *handle as i64,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            neo_riscv_abi::StackValue::ByteString(value) => {
                let bytes = value.clone().into_boxed_slice();
                let bytes_len = bytes.len();
                let bytes_ptr = Box::into_raw(bytes) as *mut u8;
                NativeStackItem {
                    kind: 1,
                    integer_value: 0,
                    bytes_ptr,
                    bytes_len,
                }
            }
            neo_riscv_abi::StackValue::Boolean(value) => NativeStackItem {
                kind: 3,
                integer_value: if *value { 1 } else { 0 },
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            neo_riscv_abi::StackValue::Array(items) => {
                let (bytes_ptr, bytes_len) = serialize_stack_items(items);
                NativeStackItem {
                    kind: 4,
                    integer_value: 0,
                    bytes_ptr: bytes_ptr.cast::<u8>(),
                    bytes_len,
                }
            }
            neo_riscv_abi::StackValue::Struct(items) => {
                let (bytes_ptr, bytes_len) = serialize_stack_items(items);
                NativeStackItem {
                    kind: 7,
                    integer_value: 0,
                    bytes_ptr: bytes_ptr.cast::<u8>(),
                    bytes_len,
                }
            }
            neo_riscv_abi::StackValue::Map(items) => {
                let flattened = items
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect::<Vec<_>>();
                let (bytes_ptr, bytes_len) = serialize_stack_items(&flattened);
                NativeStackItem {
                    kind: 8,
                    integer_value: 0,
                    bytes_ptr: bytes_ptr.cast::<u8>(),
                    bytes_len,
                }
            }
            neo_riscv_abi::StackValue::Null => NativeStackItem {
                kind: 2,
                integer_value: 0,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            neo_riscv_abi::StackValue::Pointer(value) => NativeStackItem {
                kind: 10,
                integer_value: *value,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let stack_len = native_stack.len();
    let stack_ptr = Box::into_raw(native_stack) as *mut NativeStackItem;
    (stack_ptr, stack_len)
}

fn free_native_stack_items(stack_ptr: *mut NativeStackItem, stack_len: usize) {
    if stack_ptr.is_null() {
        return;
    }

    for index in 0..stack_len {
        let item_ptr = unsafe { stack_ptr.add(index) };
        let item = unsafe { &mut *item_ptr };
        if !item.bytes_ptr.is_null() {
            if item.kind == 4 || item.kind == 7 || item.kind == 8 {
                free_native_stack_items(item.bytes_ptr.cast::<NativeStackItem>(), item.bytes_len);
            } else {
                let bytes = ptr::slice_from_raw_parts_mut(item.bytes_ptr, item.bytes_len);
                unsafe {
                    drop(Box::from_raw(bytes));
                }
            }
            item.bytes_ptr = ptr::null_mut();
            item.bytes_len = 0;
        }
    }

    let slice = ptr::slice_from_raw_parts_mut(stack_ptr, stack_len);
    unsafe {
        drop(Box::from_raw(slice));
    }
}

fn copy_native_host_result(result: &NativeHostResult) -> Result<HostCallbackResult, String> {
    if !result.error_ptr.is_null() {
        let error_bytes = unsafe { slice::from_raw_parts(result.error_ptr, result.error_len) };
        return Err(String::from_utf8_lossy(error_bytes).into_owned());
    }

    let stack = if result.stack_ptr.is_null() || result.stack_len == 0 {
        Vec::new()
    } else {
        copy_native_stack_items(result.stack_ptr, result.stack_len)?
    };

    Ok(HostCallbackResult { stack })
}

fn write_ok_result(
    result: ExecutionResult,
    fee_consumed_pico: i64,
    output: *mut NativeExecutionResult,
) {
    let (stack_ptr, stack_len) = serialize_stack_items(&result.stack);

    unsafe {
        *output = NativeExecutionResult {
            fee_consumed_pico,
            state: match result.state {
                neo_riscv_abi::VmState::Halt => 0,
                neo_riscv_abi::VmState::Fault => 1,
            },
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
}

fn write_err_result(error: String, fee_consumed_pico: i64, output: *mut NativeExecutionResult) {
    let native_error = error.into_bytes().into_boxed_slice();
    let error_len = native_error.len();
    let error_ptr = Box::into_raw(native_error) as *mut u8;

    unsafe {
        *output = NativeExecutionResult {
            fee_consumed_pico,
            state: 1,
            stack_ptr: ptr::null_mut(),
            stack_len: 0,
            error_ptr,
            error_len,
        };
    }
}

/// # Safety
///
/// - `script_ptr` must point to a valid byte buffer of `script_len` bytes.
/// - `output` must point to a valid, writable `NativeExecutionResult`.
/// - The caller owns the output and must call `neo_riscv_free_execution_result` to release it.
#[no_mangle]
pub unsafe extern "C" fn neo_riscv_execute_script(
    script_ptr: *const u8,
    script_len: usize,
    trigger: u8,
    network: u32,
    timestamp: u64,
    gas_left: i64,
    output: *mut NativeExecutionResult,
) -> bool {
    if script_ptr.is_null() || output.is_null() {
        return false;
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let script = slice::from_raw_parts(script_ptr, script_len);
        match execute_script_with_context(
            script,
            RuntimeContext {
                trigger,
                network,
                address_version: 0,
                timestamp: if timestamp == 0 {
                    None
                } else {
                    Some(timestamp)
                },
                gas_left,
                exec_fee_factor_pico: 0,
            },
        ) {
            Ok(result) => {
                write_ok_result(result, 0, output);
                true
            }
            Err(error) => {
                write_err_result(error, 0, output);
                true
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => {
            write_err_result(
                "internal panic in neo_riscv_execute_script".to_string(),
                0,
                output,
            );
            true
        }
    }
}

/// # Safety
///
/// - `script_ptr` must point to a valid byte buffer of `script_len` bytes.
/// - `initial_stack_ptr` must either be null or point to a valid `NativeStackItem` array of `initial_stack_len` elements.
/// - `output` must point to a valid, writable `NativeExecutionResult`.
/// - `callback` must be a valid function pointer that remains valid for the duration of execution.
/// - `free_callback` must be a valid function pointer for releasing host callback results.
/// - `user_data` is passed through to the callback and must satisfy its safety requirements.
/// - The caller owns the output and must call `neo_riscv_free_execution_result` to release it.
#[no_mangle]
pub unsafe extern "C" fn neo_riscv_execute_script_with_host(
    script_ptr: *const u8,
    script_len: usize,
    initial_ip: usize,
    trigger: u8,
    network: u32,
    address_version: u8,
    timestamp: u64,
    gas_left: i64,
    exec_fee_factor_pico: i64,
    initial_stack_ptr: *const NativeStackItem,
    initial_stack_len: usize,
    user_data: *mut c_void,
    callback: NativeHostCallback,
    free_callback: NativeHostFreeCallback,
    output: *mut NativeExecutionResult,
) -> bool {
    if script_ptr.is_null() || output.is_null() {
        return false;
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let script = slice::from_raw_parts(script_ptr, script_len);
        let context = RuntimeContext {
            trigger,
            network,
            address_version,
            timestamp: if timestamp == 0 {
                None
            } else {
                Some(timestamp)
            },
            gas_left,
            exec_fee_factor_pico,
        };
        let initial_stack = if initial_stack_ptr.is_null() || initial_stack_len == 0 {
            Vec::new()
        } else {
            match copy_native_stack_items(initial_stack_ptr.cast_mut(), initial_stack_len) {
                Ok(stack) => stack,
                Err(error) => {
                    write_err_result(error, 0, output);
                    return true;
                }
            }
        };
        let mut host = FfiHost {
            context,
            fee_consumed_pico: 0,
            user_data,
            callback,
            free_callback,
        };

        match execute_script_with_host_and_stack_and_ip(
            script,
            initial_stack,
            initial_ip,
            context,
            |api, ip, runtime_context, stack| {
                let mut stack_vec = stack.to_vec();
                host.context = runtime_context;
                host.syscall(api, ip, &mut stack_vec)?;
                Ok(HostCallbackResult { stack: stack_vec })
            },
        ) {
            Ok(result) => {
                let fee_consumed_pico = result.fee_consumed_pico;
                write_ok_result(result, fee_consumed_pico, output);
                true
            }
            Err(error) => {
                write_err_result(error, host.fee_consumed_pico, output);
                true
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => {
            write_err_result(
                "internal panic in neo_riscv_execute_script_with_host".to_string(),
                0,
                output,
            );
            true
        }
    }
}

/// # Safety
///
/// - `result` must either be null or point to a `NativeExecutionResult` previously returned
///   by `neo_riscv_execute_script` or `neo_riscv_execute_script_with_host`.
/// - Each result must be freed at most once.
#[no_mangle]
pub unsafe extern "C" fn neo_riscv_free_execution_result(result: *mut NativeExecutionResult) {
    if result.is_null() {
        return;
    }

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let result = &mut *result;
        if !result.stack_ptr.is_null() {
            free_native_stack_items(result.stack_ptr, result.stack_len);
            result.stack_ptr = ptr::null_mut();
            result.stack_len = 0;
        }

        if !result.error_ptr.is_null() {
            let slice = ptr::slice_from_raw_parts_mut(result.error_ptr, result.error_len);
            drop(Box::from_raw(slice));
            result.error_ptr = ptr::null_mut();
            result.error_len = 0;
        }
    }));
}

/// Execute a native RISC-V contract binary directly via PolkaVM.
///
/// # Safety
///
/// - `binary_ptr` must point to a valid byte buffer of `binary_len` bytes (the PolkaVM program).
/// - `method_ptr` must point to a valid UTF-8 byte buffer of `method_len` bytes.
/// - `initial_stack_ptr` must either be null or point to a valid `NativeStackItem` array.
/// - `output` must point to a valid, writable `NativeExecutionResult`.
/// - `callback` must be a valid function pointer that remains valid for the duration of execution.
/// - `free_callback` must be a valid function pointer for releasing host callback results.
/// - `user_data` is passed through to the callback and must satisfy its safety requirements.
/// - The caller owns the output and must call `neo_riscv_free_execution_result` to release it.
#[no_mangle]
pub unsafe extern "C" fn neo_riscv_execute_native_contract(
    binary_ptr: *const u8,
    binary_len: usize,
    method_ptr: *const u8,
    method_len: usize,
    initial_stack_ptr: *const NativeStackItem,
    initial_stack_len: usize,
    trigger: u8,
    network: u32,
    address_version: u8,
    timestamp: u64,
    gas_left: i64,
    exec_fee_factor_pico: i64,
    user_data: *mut c_void,
    callback: NativeHostCallback,
    free_callback: NativeHostFreeCallback,
    output: *mut NativeExecutionResult,
) -> bool {
    if binary_ptr.is_null() || method_ptr.is_null() || output.is_null() {
        return false;
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let binary = slice::from_raw_parts(binary_ptr, binary_len);
        let method_bytes = slice::from_raw_parts(method_ptr, method_len);
        let method = match std::str::from_utf8(method_bytes) {
            Ok(s) => s,
            Err(_) => {
                write_err_result("invalid UTF-8 method name".to_string(), 0, output);
                return true;
            }
        };
        let context = RuntimeContext {
            trigger,
            network,
            address_version,
            timestamp: if timestamp == 0 {
                None
            } else {
                Some(timestamp)
            },
            gas_left,
            exec_fee_factor_pico,
        };
        let initial_stack = if initial_stack_ptr.is_null() || initial_stack_len == 0 {
            Vec::new()
        } else {
            match copy_native_stack_items(initial_stack_ptr.cast_mut(), initial_stack_len) {
                Ok(stack) => stack,
                Err(error) => {
                    write_err_result(error, 0, output);
                    return true;
                }
            }
        };
        let mut host = FfiHost {
            context,
            fee_consumed_pico: 0,
            user_data,
            callback,
            free_callback,
        };

        match crate::execute_native_contract(
            binary,
            method,
            initial_stack,
            context,
            |api, ip, runtime_context, stack| {
                let mut stack_vec = stack.to_vec();
                host.context = runtime_context;
                host.syscall(api, ip, &mut stack_vec)?;
                Ok(crate::HostCallbackResult { stack: stack_vec })
            },
        ) {
            Ok(result) => {
                let fee_consumed_pico = result.fee_consumed_pico;
                write_ok_result(result, fee_consumed_pico, output);
                true
            }
            Err(error) => {
                write_err_result(error, host.fee_consumed_pico, output);
                true
            }
        }
    })) {
        Ok(result) => result,
        Err(_) => {
            write_err_result(
                "internal panic in neo_riscv_execute_native_contract".to_string(),
                0,
                output,
            );
            true
        }
    }
}
