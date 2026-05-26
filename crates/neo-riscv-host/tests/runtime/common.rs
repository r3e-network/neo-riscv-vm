use neo_riscv_abi::{BackendKind, StackValue, VmState};
use neo_riscv_host::{
    debug_execute_script_with_host_and_stack, execute_script, execute_script_with_context,
    execute_script_with_host, execute_script_with_host_and_stack,
    execute_script_with_host_and_stack_and_ip,
    execute_script_with_host_and_stack_and_ip_and_initializer,
    execute_script_with_host_and_stack_and_ip_and_initializer_with_result_limit,
    execute_script_with_host_and_stack_and_ip_with_result_limit, execute_script_with_trigger,
    neo_riscv_execute_script_with_host, neo_riscv_execute_script_with_host_and_initializer,
    neo_riscv_execute_script_with_host_and_initializer_and_result_limit,
    neo_riscv_free_execution_result, HostCallbackResult, NativeExecutionResult, NativeHostResult,
    PolkaVmRuntime, RuntimeContext,
};
use std::{ffi::c_void, ptr, slice};

pub fn build_native_stack_items(stack: &[StackValue]) -> (*mut neo_riscv_host::NativeStackItem, usize) {
    if stack.is_empty() {
        return (ptr::null_mut(), 0);
    }

    let native = stack
        .iter()
        .map(|value| match value {
            StackValue::Integer(value) => neo_riscv_host::NativeStackItem {
                kind: 0,
                integer_value: *value,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::ByteString(value) => {
                let bytes = value.clone().into_boxed_slice();
                let bytes_len = bytes.len();
                let bytes_ptr = Box::into_raw(bytes) as *mut u8;
                neo_riscv_host::NativeStackItem {
                    kind: 1,
                    integer_value: 0,
                    bytes_ptr,
                    bytes_len,
                }
            }
            StackValue::Boolean(value) => neo_riscv_host::NativeStackItem {
                kind: 3,
                integer_value: if *value { 1 } else { 0 },
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::Array(items) => {
                let (items_ptr, items_len) = build_native_stack_items(items);
                neo_riscv_host::NativeStackItem {
                    kind: 4,
                    integer_value: 0,
                    bytes_ptr: items_ptr.cast::<u8>(),
                    bytes_len: items_len,
                }
            }
            StackValue::Struct(items) => {
                let (items_ptr, items_len) = build_native_stack_items(items);
                neo_riscv_host::NativeStackItem {
                    kind: 7,
                    integer_value: 0,
                    bytes_ptr: items_ptr.cast::<u8>(),
                    bytes_len: items_len,
                }
            }
            StackValue::Map(items) => {
                let flattened = items
                    .iter()
                    .flat_map(|(key, value)| [key.clone(), value.clone()])
                    .collect::<Vec<_>>();
                let (items_ptr, items_len) = build_native_stack_items(&flattened);
                neo_riscv_host::NativeStackItem {
                    kind: 8,
                    integer_value: 0,
                    bytes_ptr: items_ptr.cast::<u8>(),
                    bytes_len: items_len,
                }
            }
            StackValue::Interop(handle) => neo_riscv_host::NativeStackItem {
                kind: 9,
                integer_value: *handle as i64,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::Iterator(handle) => neo_riscv_host::NativeStackItem {
                kind: 6,
                integer_value: *handle as i64,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::BigInteger(value) => {
                let bytes = value.clone().into_boxed_slice();
                let bytes_len = bytes.len();
                let bytes_ptr = Box::into_raw(bytes) as *mut u8;
                neo_riscv_host::NativeStackItem {
                    kind: 5,
                    integer_value: 0,
                    bytes_ptr,
                    bytes_len,
                }
            }
            StackValue::Null => neo_riscv_host::NativeStackItem {
                kind: 2,
                integer_value: 0,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::Pointer(value) => neo_riscv_host::NativeStackItem {
                kind: 10,
                integer_value: *value,
                bytes_ptr: ptr::null_mut(),
                bytes_len: 0,
            },
            StackValue::Buffer(value) => {
                let bytes = value.clone().into_boxed_slice();
                let bytes_len = bytes.len();
                let bytes_ptr = Box::into_raw(bytes) as *mut u8;
                neo_riscv_host::NativeStackItem {
                    kind: 11,
                    integer_value: 0,
                    bytes_ptr,
                    bytes_len,
                }
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let len = native.len();
    let ptr = Box::into_raw(native) as *mut neo_riscv_host::NativeStackItem;
    (ptr, len)
}

pub unsafe fn free_native_stack_items(ptr_items: *mut neo_riscv_host::NativeStackItem, len: usize) {
    if ptr_items.is_null() {
        return;
    }
    for index in 0..len {
        let item = unsafe { &mut *ptr_items.add(index) };
        if !item.bytes_ptr.is_null() {
            if item.kind == 4 || item.kind == 7 || item.kind == 8 {
                unsafe {
                    free_native_stack_items(
                        item.bytes_ptr.cast::<neo_riscv_host::NativeStackItem>(),
                        item.bytes_len,
                    )
                };
            } else {
                let bytes = ptr::slice_from_raw_parts_mut(item.bytes_ptr, item.bytes_len);
                unsafe { drop(Box::from_raw(bytes)) };
            }
            item.bytes_ptr = ptr::null_mut();
            item.bytes_len = 0;
        }
    }
    let slice = ptr::slice_from_raw_parts_mut(ptr_items, len);
    unsafe { drop(Box::from_raw(slice)) };
}

pub unsafe fn copy_test_native_stack_items(
    ptr_items: *mut neo_riscv_host::NativeStackItem,
    len: usize,
) -> Result<Vec<StackValue>, String> {
    let mut stack = Vec::with_capacity(len);
    for index in 0..len {
        let item = unsafe { &*ptr_items.add(index) };
        match item.kind {
            0 => stack.push(StackValue::Integer(item.integer_value)),
            1 => {
                let bytes = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe { slice::from_raw_parts(item.bytes_ptr, item.bytes_len) }.to_vec()
                };
                stack.push(StackValue::ByteString(bytes));
            }
            2 => stack.push(StackValue::Null),
            3 => stack.push(StackValue::Boolean(item.integer_value != 0)),
            5 => {
                let bytes = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe { slice::from_raw_parts(item.bytes_ptr, item.bytes_len) }.to_vec()
                };
                stack.push(StackValue::BigInteger(bytes));
            }
            4 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe {
                        copy_test_native_stack_items(
                            item.bytes_ptr.cast::<neo_riscv_host::NativeStackItem>(),
                            item.bytes_len,
                        )
                    }?
                };
                stack.push(StackValue::Array(items));
            }
            7 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe {
                        copy_test_native_stack_items(
                            item.bytes_ptr.cast::<neo_riscv_host::NativeStackItem>(),
                            item.bytes_len,
                        )
                    }?
                };
                stack.push(StackValue::Struct(items));
            }
            8 => {
                let items = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe {
                        copy_test_native_stack_items(
                            item.bytes_ptr.cast::<neo_riscv_host::NativeStackItem>(),
                            item.bytes_len,
                        )
                    }?
                };
                if items.len() % 2 != 0 {
                    return Err("map stack item contains an odd number of entries".to_string());
                }
                let mut entries = Vec::with_capacity(items.len() / 2);
                let mut iter = items.into_iter();
                while let Some(key) = iter.next() {
                    let value = iter.next().ok_or_else(|| {
                        "map stack item contains an incomplete key/value pair".to_string()
                    })?;
                    entries.push((key, value));
                }
                stack.push(StackValue::Map(entries));
            }
            6 => stack.push(StackValue::Iterator(item.integer_value as u64)),
            9 => stack.push(StackValue::Interop(item.integer_value as u64)),
            10 => stack.push(StackValue::Pointer(item.integer_value)),
            11 => {
                let bytes = if item.bytes_ptr.is_null() || item.bytes_len == 0 {
                    Vec::new()
                } else {
                    unsafe { slice::from_raw_parts(item.bytes_ptr, item.bytes_len) }.to_vec()
                };
                stack.push(StackValue::Buffer(bytes));
            }
            other => return Err(format!("unsupported native stack item kind {other}")),
        }
    }
    Ok(stack)
}

pub fn storage_context_token(id: i32, read_only: bool) -> Vec<u8> {
    let mut token = b"NRSC".to_vec();
    token.extend_from_slice(&id.to_le_bytes());
    token.push(u8::from(read_only));
    token
}

pub fn build_storage_context_round_trip_script() -> Vec<u8> {
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let put = neo_riscv_abi::interop_hash("System.Storage.Put");
    let get = neo_riscv_abi::interop_hash("System.Storage.Get");

    let mut script = Vec::new();
    script.push(0x41); // SYSCALL GetContext
    script.extend_from_slice(&get_context.to_le_bytes());
    script.push(0x4a); // DUP
    script.push(0x0c); // PUSHDATA1 "k"
    script.push(1);
    script.push(b'k');
    script.push(0x0c); // PUSHDATA1 "v"
    script.push(1);
    script.push(b'v');
    script.push(0x41); // SYSCALL Put
    script.extend_from_slice(&put.to_le_bytes());
    script.push(0x0c); // PUSHDATA1 "k"
    script.push(1);
    script.push(b'k');
    script.push(0x41); // SYSCALL Get
    script.extend_from_slice(&get.to_le_bytes());
    script.push(0x40); // RET
    script
}

