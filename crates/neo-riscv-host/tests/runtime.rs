use neo_riscv_abi::{BackendKind, StackValue, VmState};
use neo_riscv_host::{
    debug_execute_script_with_host_and_stack, execute_script, execute_script_with_context,
    execute_script_with_host, execute_script_with_host_and_stack,
    execute_script_with_host_and_stack_and_ip, execute_script_with_trigger,
    neo_riscv_execute_script_with_host, neo_riscv_free_execution_result, HostCallbackResult,
    NativeExecutionResult, NativeHostResult, PolkaVmRuntime, RuntimeContext,
};
use std::{ffi::c_void, ptr, slice};

fn build_native_stack_items(stack: &[StackValue]) -> (*mut neo_riscv_host::NativeStackItem, usize) {
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

unsafe fn free_native_stack_items(ptr_items: *mut neo_riscv_host::NativeStackItem, len: usize) {
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

unsafe fn copy_test_native_stack_items(
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

fn storage_context_token(id: i32, read_only: bool) -> Vec<u8> {
    let mut token = b"NRSC".to_vec();
    token.extend_from_slice(&id.to_le_bytes());
    token.push(u8::from(read_only));
    token
}

fn build_storage_context_round_trip_script() -> Vec<u8> {
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

#[test]
fn creates_interpreter_backed_polkavm_runtime() {
    let runtime = PolkaVmRuntime::new().expect("runtime should initialize");

    assert_eq!(runtime.backend_kind(), BackendKind::Interpreter);
}

#[test]
fn executes_push1_ret_through_host_runtime() {
    let result = execute_script(&[0x11, 0x40]).expect("host runtime should execute the script");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_runtime_platform_syscall_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result =
        execute_script(&script).expect("host runtime should execute runtime platform syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"NEO".to_vec())]);
}

#[test]
fn executes_runtime_get_trigger_syscall_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetTrigger");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_trigger(&script, 0x20)
        .expect("host runtime should execute runtime get trigger syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(0x20)]);
}

#[test]
fn caller_catch_restores_locals_after_callee_throw_in_host_runtime() {
    let script = vec![
        0x57, 0x01, 0x00, // INITSLOT 1 local, 0 args
        0x3b, 0x0c, 0x00, // TRY catch=+12, finally=0
        0x35, 0x10, 0x00, 0x00, 0x00, // CALL_L callee at ip=22
        0x45, // DROP
        0x08, // PUSHT
        0x3d, 0x08, // ENDTRY +8 -> RET at ip=21
        0x70, // STLOC0
        0x09, // PUSHF
        0x3d, 0x04, // ENDTRY +4 -> RET at ip=21
        0x38, // ABORT (unreachable filler)
        0x38, // ABORT (unreachable filler)
        0x40, // RET
        0x11, // PUSH1
        0x3a, // THROW
    ];

    let result =
        execute_script(&script).expect("host runtime should unwind into the caller catch frame");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn callt_null_result_can_flow_through_two_arg_helper_in_host_runtime() {
    let runtime_log = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let script = vec![
        0x57,
        0x01,
        0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37,
        0x02,
        0x00, // CALLT 2
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34,
        0x03, // CALL +3 -> helper at ip=13
        0x40, // RET
        0x57,
        0x00,
        0x02, // helper: INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0xd8, // ISNULL
        0x26,
        0x15, // JMPIFNOT +21 -> else branch at ip=39
        0x0c,
        0x0a,
        b'N',
        b'U',
        b'L',
        b'L',
        b' ',
        b'B',
        b'l',
        b'o',
        b'c',
        b'k', // PUSHDATA1 "NULL Block"
        0x41, // SYSCALL Runtime.Log
        (runtime_log & 0xff) as u8,
        ((runtime_log >> 8) & 0xff) as u8,
        ((runtime_log >> 16) & 0xff) as u8,
        ((runtime_log >> 24) & 0xff) as u8,
        0x0b, // PUSHNULL
        0x40, // RET
        0x08, // else: PUSHT
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(Vec::new()),
        StackValue::ByteString(vec![0x01; 32]),
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            if api == runtime_log {
                return Ok(HostCallbackResult { stack: Vec::new() });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("null CALLT result should flow through a two-arg helper");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Null]);
}

#[test]
fn callt_null_result_can_round_trip_through_local_in_host_runtime() {
    let script = vec![
        0x57, 0x01, 0x01, // INITSLOT 1 local, 1 arg
        0x78, // LDARG0
        0x37, 0x02, 0x00, // CALLT 2
        0x70, // STLOC0
        0x68, // LDLOC0
        0x40, // RET
    ];
    let initial_stack = vec![StackValue::ByteString(vec![0x01; 32])];

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("null CALLT result should survive a local store/load round-trip");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Null]);
}

#[test]
fn callt_null_result_preserves_caller_args_in_host_runtime() {
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37, 0x02, 0x00, // CALLT 2
        0x70, // STLOC0
        0x79, // LDARG1
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(Vec::new()),
        StackValue::ByteString(vec![0x01; 32]),
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("CALLT should preserve caller args");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(Vec::new())]);
}

#[test]
fn callt_null_result_can_flow_through_two_arg_helper_at_nonzero_ip_in_host_runtime() {
    let runtime_log = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let mut script = vec![0x21; 12];
    script.extend_from_slice(&[
        0x57,
        0x01,
        0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37,
        0x02,
        0x00, // CALLT 2
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34,
        0x03, // CALL +3 -> helper at ip=25
        0x40, // RET
        0x57,
        0x00,
        0x02, // helper: INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0xd8, // ISNULL
        0x26,
        0x15, // JMPIFNOT +21 -> else branch
        0x0c,
        0x0a,
        b'N',
        b'U',
        b'L',
        b'L',
        b' ',
        b'B',
        b'l',
        b'o',
        b'c',
        b'k',
        0x41,
        (runtime_log & 0xff) as u8,
        ((runtime_log >> 8) & 0xff) as u8,
        ((runtime_log >> 16) & 0xff) as u8,
        ((runtime_log >> 24) & 0xff) as u8,
        0x0b, // PUSHNULL
        0x40, // RET
        0x08, // else: PUSHT
        0x40, // RET
    ]);
    let initial_stack = vec![
        StackValue::ByteString(Vec::new()),
        StackValue::ByteString(vec![0x01; 32]),
    ];

    let result = execute_script_with_host_and_stack_and_ip(
        &script,
        initial_stack,
        12,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            if api == runtime_log {
                return Ok(HostCallbackResult { stack: Vec::new() });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("null CALLT helper flow should also pass from a nonzero entry point");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Null]);
}

#[test]
fn callt_block_like_struct_round_trips_directly_in_host_runtime() {
    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(vec![
            0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c,
            0x7d, 0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4,
            0xef, 0x1d, 0x4d, 0x1f,
        ]),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);
    let script = vec![
        0x0c, 0x20, // PUSHDATA1 32
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x37, 0x02, 0x00, // CALLT 2
        0x40, // RET
    ];
    let expected = block_like.clone();

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("block-like struct should round-trip directly through CALLT");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![block_like]);
}

#[test]
fn callt_block_like_struct_survives_local_pickitem_in_host_runtime() {
    let prev_hash = vec![
        0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c, 0x7d,
        0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4, 0xef, 0x1d,
        0x4d, 0x1f,
    ];
    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(prev_hash.clone()),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);
    let script = vec![
        0x0c, 0x20, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x57, 0x01, 0x00, // INITSLOT 1 local, 0 args
        0x37, 0x02, 0x00, // CALLT 2
        0x70, // STLOC0
        0x68, // LDLOC0
        0x12, // PUSH2
        0xCE, // PICKITEM
        0x40, // RET
    ];
    let expected = block_like.clone();

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("block-like struct should survive local PICKITEM flow");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(prev_hash)]);
}

#[test]
fn callt_block_like_struct_survives_local_pickitem_with_live_args_in_host_runtime() {
    let prev_hash = vec![
        0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c, 0x7d,
        0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4, 0xef, 0x1d,
        0x4d, 0x1f,
    ];
    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(prev_hash.clone()),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37, 0x02, 0x00, // CALLT 2
        0x70, // STLOC0
        0x68, // LDLOC0
        0x12, // PUSH2
        0xCE, // PICKITEM
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"PrevHash".to_vec()),
        StackValue::ByteString(vec![0x01; 32]),
    ];
    let expected = block_like.clone();

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _ctx, _stack| {
            if api == 0x4354_0002 {
                return Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("block-like struct should survive local PICKITEM with live caller args");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(prev_hash)]);
}

#[test]
fn helper_entry_pickitem_on_block_like_struct_arg_in_host_runtime() {
    let prev_hash = vec![
        0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c, 0x7d,
        0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4, 0xef, 0x1d,
        0x4d, 0x1f,
    ];
    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(prev_hash.clone()),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);
    let script = vec![
        0x57, 0x00, 0x02, // INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0x12, // PUSH2
        0xCE, // PICKITEM
        0x40, // RET
    ];
    let initial_stack = vec![StackValue::ByteString(b"PrevHash".to_vec()), block_like];

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |_api, _ip, _ctx, _stack| Err("unexpected callback".to_string()),
    )
    .expect("helper entry should read prev-hash from block-like struct arg");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(prev_hash)]);
}

#[test]
fn tx_like_struct_hash_then_callt_signers_in_host_runtime() {
    let tx_hash = vec![
        0xd9, 0xe0, 0xe7, 0xe0, 0x1e, 0xe5, 0x5d, 0x33, 0xee, 0x14, 0xc0, 0xda, 0x41, 0xfa, 0xe5,
        0x2a, 0x8a, 0xd4, 0x53, 0xfd, 0x6e, 0xdb, 0xdb, 0xc1, 0x47, 0x60, 0xd7, 0x4c, 0xf1, 0xc1,
        0xa1, 0xd4,
    ];
    let tx_like = StackValue::Struct(vec![
        StackValue::ByteString(tx_hash.clone()),
        StackValue::Integer(0),
        StackValue::Integer(0x01020304),
        StackValue::ByteString(vec![0x11; 20]),
        StackValue::Integer(0),
        StackValue::Integer(0),
        StackValue::Integer(0),
        StackValue::ByteString(vec![0x40]),
    ]);
    let expected_signers = StackValue::Array(vec![StackValue::Array(vec![
        StackValue::ByteString(vec![0x22; 20]),
        StackValue::Integer(0x80), // Global
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
    ])]);
    let script = vec![
        0x57, 0x00, 0x02, // INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0x10, // PUSH0
        0xCE, // PICKITEM -> tx.Hash
        0x37, 0x04, 0x00, // CALLT 4 -> getTransactionSigners
        0x40, // RET
    ];
    let initial_stack = vec![StackValue::ByteString(b"Signers".to_vec()), tx_like];
    let expected = expected_signers.clone();

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _ctx, _stack| {
            if api == 0x4354_0004 {
                return Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                });
            }
            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("tx-like struct should survive hash extraction and CALLT signers path");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![expected_signers]);
}

#[test]
fn callt_transaction_helper_then_callt_signers_with_live_args_in_host_runtime() {
    let tx_hash = vec![
        0xd9, 0xe0, 0xe7, 0xe0, 0x1e, 0xe5, 0x5d, 0x33, 0xee, 0x14, 0xc0, 0xda, 0x41, 0xfa, 0xe5,
        0x2a, 0x8a, 0xd4, 0x53, 0xfd, 0x6e, 0xdb, 0xdb, 0xc1, 0x47, 0x60, 0xd7, 0x4c, 0xf1, 0xc1,
        0xa1, 0xd4,
    ];
    let tx_like = StackValue::Struct(vec![
        StackValue::ByteString(tx_hash.clone()),
        StackValue::Integer(0),
        StackValue::Integer(0x01020304),
        StackValue::ByteString(vec![0x11; 20]),
        StackValue::Integer(0),
        StackValue::Integer(0),
        StackValue::Integer(0),
        StackValue::ByteString(vec![0x40]),
    ]);
    let expected_signers = StackValue::Array(vec![StackValue::Array(vec![
        StackValue::ByteString(vec![0x22; 20]),
        StackValue::Integer(0x80),
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
    ])]);
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37, 0x03, 0x00, // CALLT 3 -> getTransaction
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34, 0x06, // CALL +6 -> helper
        0x37, 0x04, 0x00, // CALLT 4 -> getTransactionSigners
        0x40, // RET
        0x57, 0x00, 0x02, // helper
        0x78, // LDARG0
        0x10, // PUSH0
        0xCE, // PICKITEM -> tx.Hash
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"Signers".to_vec()),
        StackValue::ByteString(tx_hash.clone()),
    ];
    let expected = expected_signers.clone();
    let expected_tx = tx_like.clone();

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _ctx, _stack| match api {
            0x4354_0003 => Ok(HostCallbackResult {
                stack: vec![expected_tx.clone()],
            }),
            0x4354_0004 => Ok(HostCallbackResult {
                stack: vec![expected.clone()],
            }),
            _ => Err(format!("unexpected callback api 0x{api:08x}")),
        },
    )
    .expect("tx helper -> CALLT signers flow should survive with live args");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![expected_signers]);
}

#[test]
fn local_block_like_struct_then_helper_pickitem_in_host_runtime() {
    let prev_hash = vec![
        0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c, 0x7d,
        0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4, 0xef, 0x1d,
        0x4d, 0x1f,
    ];
    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(prev_hash.clone()),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34, 0x03, // CALL +3 -> helper
        0x40, // RET
        0x57, 0x00, 0x02, // helper
        0x78, // LDARG0
        0x12, // PUSH2
        0xCE, // PICKITEM
        0x40, // RET
    ];
    let initial_stack = vec![StackValue::ByteString(b"PrevHash".to_vec()), block_like];

    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |_api, _ip, _ctx, _stack| Err("unexpected callback".to_string()),
    )
    .expect("local struct -> helper PICKITEM flow should preserve prev-hash");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(prev_hash)]);
}

#[test]
fn executes_runtime_get_network_syscall_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetNetwork");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_context(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860_833_102,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
    )
    .expect("host runtime should execute runtime get network syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(860_833_102)]);
}

#[test]
fn executes_runtime_gas_left_syscall_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GasLeft");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_context(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 123_456,
            exec_fee_factor_pico: 0,
        },
    )
    .expect("host runtime should execute runtime gas left syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(123_456)]);
}

#[test]
fn executes_platform_syscall_through_custom_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == neo_riscv_abi::interop_hash("System.Runtime.Platform") {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"R3E".to_vec())],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should execute platform syscall through callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"R3E".to_vec())]);
}

#[test]
fn executes_null_stack_item_through_custom_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetCallingScriptHash");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == neo_riscv_abi::interop_hash("System.Runtime.GetCallingScriptHash") {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should execute null stack item callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Null]);
}

#[test]
fn executes_array_stack_item_through_custom_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.CurrentSigners");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == syscall {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::Array(vec![
                        StackValue::ByteString(vec![1; 20]),
                        StackValue::Integer(1),
                        StackValue::Array(vec![]),
                        StackValue::Array(vec![]),
                        StackValue::Array(vec![]),
                    ])])],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should execute array stack item callback");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![StackValue::Array(vec![
            StackValue::ByteString(vec![1; 20]),
            StackValue::Integer(1),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
        ])])]
    );
}

#[test]
fn executes_struct_stack_item_through_custom_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Storage.Find");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == syscall {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Struct(vec![
                        StackValue::ByteString(vec![0x01]),
                        StackValue::ByteString(vec![0x02]),
                    ])],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should execute struct stack item callback");

    assert_eq!(
        result.stack,
        vec![StackValue::Struct(vec![
            StackValue::ByteString(vec![0x01]),
            StackValue::ByteString(vec![0x02]),
        ])]
    );
}

#[test]
fn notifications_like_result_round_trips_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetNotifications");
    let mut script = vec![0x0b]; // PUSHNULL
    script.push(0x41);
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let expected = StackValue::Array(vec![
        StackValue::Array(vec![
            StackValue::ByteString(vec![0x11; 20]),
            StackValue::ByteString(b"testEvent1".to_vec()),
            StackValue::Array(vec![]),
        ]),
        StackValue::Array(vec![
            StackValue::ByteString(vec![0x22; 20]),
            StackValue::ByteString(b"testEvent2".to_vec()),
            StackValue::Array(vec![StackValue::Integer(1)]),
        ]),
    ]);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == syscall {
                Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("notification-shaped callback payload should survive round trip");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![expected]);
}

#[test]
fn contract_state_like_result_round_trips_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let expected = StackValue::Array(vec![
        StackValue::Integer(-6),
        StackValue::Integer(0),
        StackValue::ByteString(vec![0x33; 20]),
        StackValue::ByteString(vec![0x44; 162]),
        StackValue::Struct(vec![
            StackValue::ByteString(b"NeoToken".to_vec()),
            StackValue::Array(vec![]),
            StackValue::Map(vec![]),
            StackValue::Array(vec![StackValue::ByteString(b"NEP-17".to_vec())]),
            StackValue::Struct(vec![
                StackValue::Array(vec![StackValue::Struct(vec![
                    StackValue::ByteString(b"balanceOf".to_vec()),
                    StackValue::Array(vec![StackValue::Struct(vec![
                        StackValue::ByteString(b"account".to_vec()),
                        StackValue::Integer(18),
                    ])]),
                    StackValue::Integer(17),
                    StackValue::Integer(7),
                    StackValue::Boolean(true),
                ])]),
                StackValue::Array(vec![StackValue::Struct(vec![
                    StackValue::ByteString(b"Transfer".to_vec()),
                    StackValue::Array(vec![
                        StackValue::Struct(vec![
                            StackValue::ByteString(b"from".to_vec()),
                            StackValue::Integer(18),
                        ]),
                        StackValue::Struct(vec![
                            StackValue::ByteString(b"to".to_vec()),
                            StackValue::Integer(18),
                        ]),
                        StackValue::Struct(vec![
                            StackValue::ByteString(b"amount".to_vec()),
                            StackValue::Integer(17),
                        ]),
                    ]),
                ])]),
            ]),
            StackValue::Array(vec![StackValue::Struct(vec![
                StackValue::ByteString(vec![0x55; 20]),
                StackValue::Array(vec![StackValue::ByteString(b"transfer".to_vec())]),
            ])]),
            StackValue::Null,
            StackValue::ByteString(b"null".to_vec()),
        ]),
    ]);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == syscall {
                Ok(HostCallbackResult {
                    stack: vec![expected.clone()],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("contract-state-shaped callback payload should survive round trip");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![expected]);
}

#[test]
fn custom_host_callback_receives_input_stack_for_log_style_syscall() {
    let api = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let script = vec![
        0x0c,
        0x05,
        b'h',
        b'e',
        b'l',
        b'l',
        b'o',
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];
    let mut observed_message = None;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed_message = match stack.last() {
                Some(StackValue::ByteString(value)) => {
                    Some(String::from_utf8_lossy(value.as_slice()).into_owned())
                }
                _ => None,
            };

            let mut next_stack = stack.to_vec();
            next_stack.pop();
            Ok(HostCallbackResult { stack: next_stack })
        },
    )
    .expect("host runtime should execute log-style syscall through callback");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
    assert_eq!(observed_message.as_deref(), Some("hello"));
}

#[test]
fn custom_host_callback_receives_current_instruction_pointer() {
    let api = neo_riscv_abi::interop_hash("System.Contract.CallNative");
    let script = vec![
        0x11,
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];
    let mut observed_ip = None;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed_ip = Some(ip);
            let mut next_stack = stack.to_vec();
            next_stack.pop();
            Ok(HostCallbackResult { stack: next_stack })
        },
    )
    .expect("host runtime should expose the syscall instruction pointer to custom callbacks");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
    assert_eq!(observed_ip, Some(1));
}

#[test]
fn executes_runtime_get_time_syscall_through_host_runtime() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetTime");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_context(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: Some(1_710_000_000),
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
    )
    .expect("host runtime should execute runtime get time syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1_710_000_000)]);
}

#[test]
fn runtime_get_time_syscall_faults_without_timestamp() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.GetTime");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let error = execute_script_with_context(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
    )
    .expect_err("host runtime should fault when runtime get time has no timestamp");

    assert!(
        error.contains("GetTime")
            || error.contains("host syscall failed")
            || error.contains("Failed to decode result")
    );
}

#[test]
fn polkavm_execution_reports_opcode_fee_consumed() {
    // The PolkaVM guest now reports each executed NeoVM opcode back to the
    // host so fee accounting stays aligned with the C# engine.
    let result = execute_script_with_context(
        &[0x11, 0x40],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 100_000,
            exec_fee_factor_pico: 10_000,
        },
    )
    .expect("host runtime should execute with opcode fee accounting in the PolkaVM path");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
    assert_eq!(
        result.fee_consumed_pico, 10_000,
        "PUSH1 should consume one datoshi at the configured fee factor"
    );
}

#[test]
fn custom_host_callback_handles_large_bytestring_array_argument() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);
    let expected = vec![StackValue::Array(vec![
        StackValue::ByteString(vec![0x42; 65_536]),
        StackValue::ByteString(vec![0x24; 32]),
        StackValue::Null,
    ])];
    let mut observed = None;

    let result = execute_script_with_host_and_stack(
        &script,
        expected.clone(),
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed = Some(stack.to_vec());
            Ok(HostCallbackResult { stack: Vec::new() })
        },
    )
    .expect("host runtime should pass large byte-string arrays through the guest boundary");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
    assert_eq!(observed, Some(expected));
}

#[test]
fn custom_host_callback_handles_large_dynamic_call_shape_with_prior_stack_item() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);
    let expected = vec![
        StackValue::Integer(1),
        StackValue::Array(vec![StackValue::ByteString(vec![0x42; 576])]),
        StackValue::Integer(i64::from(0x0f_u8)),
        StackValue::ByteString(b"bls12381Deserialize".to_vec()),
        StackValue::ByteString(vec![0x55; 20]),
    ];
    let mut observed = None;

    let result = execute_script_with_host_and_stack(
        &script,
        expected.clone(),
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed = Some(stack.to_vec());
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(2)],
            })
        },
    )
    .expect("host runtime should handle a large dynamic-call shaped stack with a prior item");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
    assert_eq!(
        observed,
        Some(vec![
            StackValue::Array(vec![StackValue::ByteString(vec![0x42; 576])]),
            StackValue::Integer(i64::from(0x0f_u8)),
            StackValue::ByteString(b"bls12381Deserialize".to_vec()),
            StackValue::ByteString(vec![0x55; 20]),
        ])
    );
}

#[test]
fn large_dynamic_call_host_error_surfaces_without_trap() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);
    let initial_stack = vec![
        StackValue::Array(vec![StackValue::ByteString(vec![0x42; 65_536])]),
        StackValue::Integer(i64::from(0x0f_u8)),
        StackValue::ByteString(b"deploy".to_vec()),
        StackValue::ByteString(vec![0x55; 20]),
    ];

    let error = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Err("simulated large-call host error".to_string())
        },
    )
    .expect_err("large dynamic call host errors should propagate cleanly");

    assert!(
        error.contains("simulated large-call host error"),
        "expected original host error, got: {error}"
    );
}

#[test]
fn large_dynamic_call_wrapper_host_error_surfaces_without_trap() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();
    script.push(0x0b); // PUSHNULL
    script.push(0x0e); // PUSHDATA4
    script.extend_from_slice(&(65_536u32).to_le_bytes());
    script.extend_from_slice(&vec![0x42; 65_536]);
    script.push(0x0c); // PUSHDATA1
    script.push(1);
    script.push(0xaa);
    script.push(0x13); // PUSH3
    script.push(0xc0); // PACK
    script.push(0x1f); // PUSH15 (CallFlags.All)
    script.push(0x0c); // PUSHDATA1
    script.push(6);
    script.extend_from_slice(b"deploy");
    script.push(0x0c); // PUSHDATA1
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40); // RET

    let error = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Err("simulated wrapped host error".to_string())
        },
    )
    .expect_err("large dynamic call wrapper host errors should propagate cleanly");

    assert!(
        error.contains("simulated wrapped host error"),
        "expected original host error, got: {error}"
    );
}

#[test]
fn large_dynamic_call_wrapper_host_error_surfaces_without_trap_with_fee_accounting() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();
    script.push(0x0b); // PUSHNULL
    script.push(0x0e); // PUSHDATA4
    script.extend_from_slice(&(65_536u32).to_le_bytes());
    script.extend_from_slice(&vec![0x42; 65_536]);
    script.push(0x0c); // PUSHDATA1
    script.push(1);
    script.push(0xaa);
    script.push(0x13); // PUSH3
    script.push(0xc0); // PACK
    script.push(0x1f); // PUSH15 (CallFlags.All)
    script.push(0x0c); // PUSHDATA1
    script.push(6);
    script.extend_from_slice(b"deploy");
    script.push(0x0c); // PUSHDATA1
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40); // RET

    let error = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 20_000_000_000,
            exec_fee_factor_pico: 300_000,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Err("simulated wrapped host error with fees".to_string())
        },
    )
    .expect_err("large wrapped host errors should propagate cleanly with fee accounting");

    assert!(
        error.contains("simulated wrapped host error with fees"),
        "expected original host error, got: {error}"
    );
}

#[test]
fn dynamic_call_wrapper_executes_from_nonempty_stack_with_large_argument() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = Vec::new();
    script.push(0x0d);
    script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
    script.extend_from_slice(&gt);
    script.push(0x11);
    script.push(0xc0);
    script.push(0x1f);
    script.push(0x0c);
    script.push(19);
    script.extend_from_slice(b"bls12381Deserialize");
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);

    let mut observed = None;
    let result = execute_script_with_host_and_stack(
        &script,
        vec![StackValue::Integer(1)],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            observed = Some(stack.to_vec());
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(2)],
            })
        },
    )
    .expect("dynamic call wrapper should execute from a non-empty stack");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
    assert!(observed.is_some());
}

#[test]
fn dynamic_call_wrapper_executes_after_dropping_large_argument_array() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = Vec::new();
    script.push(0x0d);
    script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
    script.extend_from_slice(&gt);
    script.push(0x11);
    script.push(0xc0);
    script.push(0x45);
    script.push(0x0d);
    script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
    script.extend_from_slice(&gt);
    script.push(0x11);
    script.push(0xc0);
    script.push(0x1f);
    script.push(0x0c);
    script.push(19);
    script.extend_from_slice(b"bls12381Deserialize");
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(2)],
            })
        },
    )
    .expect("dynamic call wrapper should execute after dropping a large array");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn large_dynamic_call_executes_after_small_prior_syscall() {
    let small_api = 0x01020304u32;
    let large_api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = vec![0x41];
    script.extend_from_slice(&small_api.to_le_bytes());
    script.push(0x0d);
    script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
    script.extend_from_slice(&gt);
    script.push(0x11);
    script.push(0xc0);
    script.push(0x1f);
    script.push(0x0c);
    script.push(19);
    script.extend_from_slice(b"bls12381Deserialize");
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&large_api.to_le_bytes());
    script.push(0x40);

    let mut call_count = 0u32;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            call_count += 1;
            Ok(HostCallbackResult {
                stack: match callback_api {
                    api if api == small_api => vec![StackValue::Integer(1)],
                    api if api == large_api => vec![StackValue::Integer(2)],
                    other => panic!("unexpected syscall 0x{other:08x}"),
                },
            })
        },
    )
    .expect("large dynamic call should execute after a small prior syscall");

    assert_eq!(call_count, 2);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
}

#[test]
fn second_large_dynamic_call_executes_if_first_result_is_dropped() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = Vec::new();
    for index in 0..2 {
        script.push(0x0d);
        script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
        script.extend_from_slice(&gt);
        script.push(0x11);
        script.push(0xc0);
        script.push(0x1f);
        script.push(0x0c);
        script.push(19);
        script.extend_from_slice(b"bls12381Deserialize");
        script.push(0x0c);
        script.push(20);
        script.extend_from_slice(&[0x55; 20]);
        script.push(0x41);
        script.extend_from_slice(&api.to_le_bytes());
        if index == 0 {
            script.push(0x45); // DROP first result
        }
    }
    script.push(0x40);

    let mut call_count = 0u32;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            call_count += 1;
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(call_count as i64)],
            })
        },
    )
    .expect("second large dynamic call should work when the first result is dropped");

    assert_eq!(call_count, 2);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn contract_call_bool_result_survives_heap_backed_locals_after_syscall() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();

    // INITSLOT 7 locals, 0 args
    script.extend_from_slice(&[0x57, 0x07, 0x00]);

    // LOC0 = [sig]
    script.extend_from_slice(&[0x0c, 0x40]);
    script.extend_from_slice(&[0x11; 64]);
    script.extend_from_slice(&[0x11, 0xc0, 0x70]);

    // LOC1 = [pub]
    script.extend_from_slice(&[0x0c, 0x21]);
    script.extend_from_slice(&[0x22; 33]);
    script.extend_from_slice(&[0x11, 0xc0, 0x71]);

    // LOC2 = CAT([0x33;4], [0x44;32]) => Buffer(36)
    script.extend_from_slice(&[0x0c, 0x04]);
    script.extend_from_slice(&[0x33; 4]);
    script.extend_from_slice(&[0x0c, 0x20]);
    script.extend_from_slice(&[0x44; 32]);
    script.extend_from_slice(&[0x8b, 0x72]);

    // sigCnt=0, pubCnt=0, n=1, m=1
    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74, 0x11, 0x75, 0x11, 0x76]);

    let loop_start = script.len();

    // if sigCnt >= m || pubCnt >= n jump to end
    script.extend_from_slice(&[0x6b, 0x6e, 0xb8, 0x6c, 0x6d, 0xb8, 0x92, 0x24, 0x00]);
    let jmpif_offset_index = script.len() - 1;

    // Build args and call System.Contract.Call
    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());

    // sigCnt += result; pubCnt++
    script.extend_from_slice(&[0x6b, 0x9e, 0x73, 0x6c, 0x9c, 0x74]);

    // JMP loop_start
    script.extend_from_slice(&[0x22, 0x00]);
    let jmp_back_offset_index = script.len() - 1;

    let end_offset = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb3, 0x40]); // LDLOC3, LDLOC6, NUMEQUAL, RET

    script[jmpif_offset_index] = (end_offset as isize - jmpif_offset_index as isize + 1) as u8;
    script[jmp_back_offset_index] =
        (loop_start as isize - jmp_back_offset_index as isize + 1) as i8 as u8;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            assert_eq!(stack.len(), 4);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Boolean(true)],
            })
        },
    )
    .expect("guest should survive LDLOC continuation after contract-call callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn contract_call_bool_result_survives_multisig_like_heap_locals_after_syscall() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();

    // INITSLOT 7 locals, 0 args
    script.extend_from_slice(&[0x57, 0x07, 0x00]);

    // LOC0 = [sig0, sig1, sig2]
    for fill in [0x11_u8, 0x12, 0x13] {
        script.extend_from_slice(&[0x0c, 0x40]);
        script.extend_from_slice(&[fill; 64]);
    }
    script.extend_from_slice(&[0x13, 0xc0, 0x70]);

    // LOC1 = [pub0, pub1, pub2, pub3]
    for fill in [0x21_u8, 0x22, 0x23, 0x24] {
        script.extend_from_slice(&[0x0c, 0x21]);
        script.extend_from_slice(&[fill; 33]);
    }
    script.extend_from_slice(&[0x14, 0xc0, 0x71]);

    // LOC2 = CAT([0x33;4], [0x44;32]) => Buffer(36)
    script.extend_from_slice(&[0x0c, 0x04]);
    script.extend_from_slice(&[0x33; 4]);
    script.extend_from_slice(&[0x0c, 0x20]);
    script.extend_from_slice(&[0x44; 32]);
    script.extend_from_slice(&[0x8b, 0x72]);

    // sigCnt=0, pubCnt=0, n=4, m=3
    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74, 0x14, 0x75, 0x13, 0x76]);

    let loop_start = script.len();

    // if sigCnt >= m || pubCnt >= n jump to end
    script.extend_from_slice(&[0x6b, 0x6e, 0xb8, 0x6c, 0x6d, 0xb8, 0x92, 0x24, 0x00]);
    let jmpif_offset_index = script.len() - 1;

    // Build args and call System.Contract.Call
    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());

    // sigCnt += result; pubCnt++
    script.extend_from_slice(&[0x6b, 0x9e, 0x73, 0x6c, 0x9c, 0x74]);

    // JMP loop_start
    script.extend_from_slice(&[0x22, 0x00]);
    let jmp_back_offset_index = script.len() - 1;

    let end_offset = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb3, 0x40]); // LDLOC3, LDLOC6, NUMEQUAL, RET

    script[jmpif_offset_index] = (end_offset as isize - jmpif_offset_index as isize + 1) as u8;
    script[jmp_back_offset_index] =
        (loop_start as isize - jmp_back_offset_index as isize + 1) as i8 as u8;

    let mut callback_count = 0;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            assert_eq!(stack.len(), 4);
            callback_count += 1;
            Ok(HostCallbackResult {
                stack: vec![StackValue::Boolean(true)],
            })
        },
    )
    .expect(
        "guest should survive the multisig-like LDLOC continuation after contract-call callbacks",
    );

    assert_eq!(callback_count, 3);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn contract_call_bool_result_survives_multisig_like_initial_stack_across_contexts() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();

    // Initial stack supplies the 3 signatures, like a verification script after invocation.
    // Script then builds locals the same way as the real multisig witness.
    script.push(0x13); // PUSH3 (m)
    for fill in [0x21_u8, 0x22, 0x23, 0x24] {
        script.extend_from_slice(&[0x0c, 0x21]);
        script.extend_from_slice(&[fill; 33]);
    }
    script.push(0x14); // PUSH4 (n)
    script.extend_from_slice(&[0x57, 0x07, 0x00]); // INITSLOT 7 0
    script.push(0x75); // STLOC5 (n)
    script.extend_from_slice(&[0x6d, 0xc0, 0x71]); // LDLOC5 PACK STLOC1
    script.push(0x76); // STLOC6 (m)
    script.extend_from_slice(&[0x6e, 0xc0, 0x70]); // LDLOC6 PACK STLOC0

    // LOC2 = CAT([0x33;4], [0x44;32]) => Buffer(36)
    script.extend_from_slice(&[0x0c, 0x04]);
    script.extend_from_slice(&[0x33; 4]);
    script.extend_from_slice(&[0x0c, 0x20]);
    script.extend_from_slice(&[0x44; 32]);
    script.extend_from_slice(&[0x8b, 0x72]);

    // sigCnt=0, pubCnt=0
    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74]);

    let loop_start = script.len();

    // if sigCnt >= m || pubCnt >= n jump to end
    script.extend_from_slice(&[0x6b, 0x6e, 0xb8, 0x6c, 0x6d, 0xb8, 0x92, 0x24, 0x00]);
    let jmpif_offset_index = script.len() - 1;

    // Build args and call System.Contract.Call
    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());

    // sigCnt += result; pubCnt++
    script.extend_from_slice(&[0x6b, 0x9e, 0x73, 0x6c, 0x9c, 0x74]);

    // JMP loop_start
    script.extend_from_slice(&[0x22, 0x00]);
    let jmp_back_offset_index = script.len() - 1;

    let end_offset = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb3, 0x40]); // LDLOC3, LDLOC6, NUMEQUAL, RET

    script[jmpif_offset_index] = (end_offset as isize - jmpif_offset_index as isize + 1) as u8;
    script[jmp_back_offset_index] =
        (loop_start as isize - jmp_back_offset_index as isize + 1) as i8 as u8;

    let initial_stack = vec![
        StackValue::ByteString(vec![0x11; 64]),
        StackValue::ByteString(vec![0x12; 64]),
        StackValue::ByteString(vec![0x13; 64]),
    ];
    let mut callback_count = 0;
    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            assert_eq!(stack.len(), 4);
            callback_count += 1;
            Ok(HostCallbackResult {
                stack: vec![StackValue::Boolean(true)],
            })
        },
    )
    .expect("guest should survive the multisig-like initial-stack continuation after contract-call callbacks");

    assert_eq!(callback_count, 3);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn contract_call_bool_result_survives_multisig_like_scriptcontainer_path() {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    let get_network = neo_riscv_abi::interop_hash("System.Runtime.GetNetwork");
    let get_script_container = neo_riscv_abi::interop_hash("System.Runtime.GetScriptContainer");
    let mut script = Vec::new();

    // Initial stack supplies the 3 signatures, like a verification script after invocation.
    script.push(0x13); // PUSH3 (m)
    for fill in [0x21_u8, 0x22, 0x23, 0x24] {
        script.extend_from_slice(&[0x0c, 0x21]);
        script.extend_from_slice(&[fill; 33]);
    }
    script.push(0x14); // PUSH4 (n)
    script.extend_from_slice(&[0x57, 0x07, 0x00]); // INITSLOT 7 0
    script.push(0x75); // STLOC5 (n)
    script.extend_from_slice(&[0x6d, 0xc0, 0x71]); // LDLOC5 PACK STLOC1
    script.push(0x76); // STLOC6 (m)
    script.extend_from_slice(&[0x6e, 0xc0, 0x70]); // LDLOC6 PACK STLOC0

    // LOC2 = CAT(LEFT(GetNetwork()+0x100000000, 4), GetScriptContainer()[0])
    script.push(0x41);
    script.extend_from_slice(&get_network.to_le_bytes());
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&0x0000_0001_0000_0000_i64.to_le_bytes());
    script.extend_from_slice(&[0x9e, 0x14, 0x8d]); // ADD, PUSH4, LEFT
    script.push(0x41);
    script.extend_from_slice(&get_script_container.to_le_bytes());
    script.extend_from_slice(&[0x10, 0xce, 0x8b, 0x72]); // PUSH0, PICKITEM, CAT, STLOC2

    // sigCnt=0, pubCnt=0
    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74]);

    let loop_start = script.len();

    // if sigCnt >= m || pubCnt >= n jump to end
    script.extend_from_slice(&[0x6b, 0x6e, 0xb8, 0x6c, 0x6d, 0xb8, 0x92, 0x24, 0x00]);
    let jmpif_offset_index = script.len() - 1;

    // Build args and call System.Contract.Call
    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&contract_call.to_le_bytes());

    // sigCnt += result; pubCnt++
    script.extend_from_slice(&[0x6b, 0x9e, 0x73, 0x6c, 0x9c, 0x74]);

    // JMP loop_start
    script.extend_from_slice(&[0x22, 0x00]);
    let jmp_back_offset_index = script.len() - 1;

    let end_offset = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb3, 0x40]); // LDLOC3, LDLOC6, NUMEQUAL, RET

    script[jmpif_offset_index] = (end_offset as isize - jmpif_offset_index as isize + 1) as u8;
    script[jmp_back_offset_index] =
        (loop_start as isize - jmp_back_offset_index as isize + 1) as i8 as u8;

    let initial_stack = vec![
        StackValue::ByteString(vec![0x11; 64]),
        StackValue::ByteString(vec![0x12; 64]),
        StackValue::ByteString(vec![0x13; 64]),
    ];
    let mut contract_call_count = 0;
    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| match callback_api {
            api if api == get_network => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(860_833_102)],
                })
            }
            api if api == get_script_container => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(vec![0x77; 32])])],
                })
            }
            api if api == contract_call => {
                assert_eq!(stack.len(), 4);
                contract_call_count += 1;
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                })
            }
            other => panic!("unexpected syscall 0x{other:08x}"),
        },
    )
    .expect("guest should survive the multisig-like script-container continuation after contract-call callbacks");

    assert_eq!(contract_call_count, 3);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn contract_call_bool_result_survives_multisig_like_full_transaction_scriptcontainer_path() {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    let get_network = neo_riscv_abi::interop_hash("System.Runtime.GetNetwork");
    let get_script_container = neo_riscv_abi::interop_hash("System.Runtime.GetScriptContainer");
    let mut script = Vec::new();

    script.push(0x13); // PUSH3 (m)
    for fill in [0x21_u8, 0x22, 0x23, 0x24] {
        script.extend_from_slice(&[0x0c, 0x21]);
        script.extend_from_slice(&[fill; 33]);
    }
    script.push(0x14); // PUSH4 (n)
    script.extend_from_slice(&[0x57, 0x07, 0x00]); // INITSLOT 7 0
    script.push(0x75); // STLOC5 (n)
    script.extend_from_slice(&[0x6d, 0xc0, 0x71]); // LDLOC5 PACK STLOC1
    script.push(0x76); // STLOC6 (m)
    script.extend_from_slice(&[0x6e, 0xc0, 0x70]); // LDLOC6 PACK STLOC0

    // LOC2 = CAT(LEFT(GetNetwork()+0x100000000, 4), GetScriptContainer()[0])
    script.push(0x41);
    script.extend_from_slice(&get_network.to_le_bytes());
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&0x0000_0001_0000_0000_i64.to_le_bytes());
    script.extend_from_slice(&[0x9e, 0x14, 0x8d]); // ADD, PUSH4, LEFT
    script.push(0x41);
    script.extend_from_slice(&get_script_container.to_le_bytes());
    script.extend_from_slice(&[0x10, 0xce, 0x8b, 0x72]); // PUSH0, PICKITEM, CAT, STLOC2

    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74]); // sigCnt=0, pubCnt=0

    let loop_start = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb8, 0x6c, 0x6d, 0xb8, 0x92, 0x24, 0x00]);
    let jmpif_offset_index = script.len() - 1;

    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&contract_call.to_le_bytes());

    script.extend_from_slice(&[0x6b, 0x9e, 0x73, 0x6c, 0x9c, 0x74]); // sigCnt += result; pubCnt++

    script.extend_from_slice(&[0x22, 0x00]); // JMP loop_start
    let jmp_back_offset_index = script.len() - 1;

    let end_offset = script.len();
    script.extend_from_slice(&[0x6b, 0x6e, 0xb3, 0x40]); // LDLOC3, LDLOC6, NUMEQUAL, RET

    script[jmpif_offset_index] = (end_offset as isize - jmpif_offset_index as isize + 1) as u8;
    script[jmp_back_offset_index] =
        (loop_start as isize - jmp_back_offset_index as isize + 1) as i8 as u8;

    let initial_stack = vec![
        StackValue::ByteString(vec![0x11; 64]),
        StackValue::ByteString(vec![0x12; 64]),
        StackValue::ByteString(vec![0x13; 64]),
    ];
    let mut contract_call_count = 0;
    let result = execute_script_with_host_and_stack(
        &script,
        initial_stack,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| match callback_api {
            api if api == get_network => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(860_833_102)],
                })
            }
            api if api == get_script_container => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![
                        StackValue::ByteString(vec![0x77; 32]),  // hash
                        StackValue::Integer(0),                  // version
                        StackValue::Integer(1),                  // nonce
                        StackValue::ByteString(vec![0x33; 20]),  // sender
                        StackValue::Integer(0),                  // system fee
                        StackValue::Integer(100_000_000),        // network fee
                        StackValue::Integer(10),                 // valid until block
                        StackValue::ByteString(vec![0xAA; 1024]), // script
                    ])],
                })
            }
            api if api == contract_call => {
                assert_eq!(stack.len(), 4);
                contract_call_count += 1;
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                })
            }
            other => panic!("unexpected syscall 0x{other:08x}"),
        },
    )
    .expect(
        "guest should survive the multisig-like continuation with a full transaction-shaped script-container payload",
    );

    assert_eq!(contract_call_count, 3);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn full_transaction_scriptcontainer_path_preserves_integer_local_without_following_host_call() {
    let get_network = neo_riscv_abi::interop_hash("System.Runtime.GetNetwork");
    let get_script_container = neo_riscv_abi::interop_hash("System.Runtime.GetScriptContainer");
    let mut script = Vec::new();

    script.extend_from_slice(&[0x57, 0x04, 0x00]); // INITSLOT 4 0
    script.extend_from_slice(&[0x10, 0x73]); // PUSH0 STLOC3

    // Call GetNetwork and GetScriptContainer, then drop both returned stack items.
    script.push(0x41);
    script.extend_from_slice(&get_network.to_le_bytes());
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&0x0000_0001_0000_0000_i64.to_le_bytes());
    script.extend_from_slice(&[0x9e, 0x14, 0x8d]); // ADD, PUSH4, LEFT
    script.push(0x41);
    script.extend_from_slice(&get_script_container.to_le_bytes());
    script.extend_from_slice(&[0x45, 0x45]); // DROP array, DROP prefix bytes

    script.extend_from_slice(&[0x6b, 0x40]); // LDLOC3, RET

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| match callback_api {
            api if api == get_network => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(860_833_102)],
                })
            }
            api if api == get_script_container => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![
                        StackValue::ByteString(vec![0x77; 32]),
                        StackValue::Integer(0),
                        StackValue::Integer(1),
                        StackValue::ByteString(vec![0x33; 20]),
                        StackValue::Integer(0),
                        StackValue::Integer(100_000_000),
                        StackValue::Integer(10),
                        StackValue::ByteString(vec![0xAA; 1024]),
                    ])],
                })
            }
            other => panic!("unexpected syscall 0x{other:08x}"),
        },
    )
    .expect("full transaction-shaped script-container path should preserve later integer locals");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(0)]);
}

#[test]
fn contract_call_bool_result_after_full_transaction_scriptcontainer_survives_single_iteration() {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    let get_network = neo_riscv_abi::interop_hash("System.Runtime.GetNetwork");
    let get_script_container = neo_riscv_abi::interop_hash("System.Runtime.GetScriptContainer");
    let mut script = Vec::new();

    script.extend_from_slice(&[0x57, 0x07, 0x00]); // INITSLOT 7 0

    // LOC0 = [sig]
    script.extend_from_slice(&[0x0c, 0x40]);
    script.extend_from_slice(&[0x11; 64]);
    script.extend_from_slice(&[0x11, 0xc0, 0x70]);

    // LOC1 = [pub]
    script.extend_from_slice(&[0x0c, 0x21]);
    script.extend_from_slice(&[0x22; 33]);
    script.extend_from_slice(&[0x11, 0xc0, 0x71]);

    // LOC2 = CAT(LEFT(GetNetwork()+0x100000000, 4), GetScriptContainer()[0])
    script.push(0x41);
    script.extend_from_slice(&get_network.to_le_bytes());
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&0x0000_0001_0000_0000_i64.to_le_bytes());
    script.extend_from_slice(&[0x9e, 0x14, 0x8d]); // ADD, PUSH4, LEFT
    script.push(0x41);
    script.extend_from_slice(&get_script_container.to_le_bytes());
    script.extend_from_slice(&[0x10, 0xce, 0x8b, 0x72]); // PUSH0, PICKITEM, CAT, STLOC2

    // sigCnt=0, pubCnt=0, n=1, m=1
    script.extend_from_slice(&[0x10, 0x73, 0x10, 0x74, 0x11, 0x75, 0x11, 0x76]);

    // Build args and call System.Contract.Call once.
    script.extend_from_slice(&[0x00, 0x7a]); // PUSHINT8 122
    script.extend_from_slice(&[0x68, 0x6b, 0xce]); // LDLOC0, LDLOC3, PICKITEM
    script.extend_from_slice(&[0x69, 0x6c, 0xce]); // LDLOC1, LDLOC4, PICKITEM
    script.extend_from_slice(&[0x6a, 0x14, 0xc0]); // LDLOC2, PUSH4, PACK
    script.push(0x10); // PUSH0 call flags
    script.extend_from_slice(&[0x0c, 0x0f]);
    script.extend_from_slice(b"verifyWithECDsa");
    script.extend_from_slice(&[0x0c, 0x14]);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&contract_call.to_le_bytes());

    script.extend_from_slice(&[0x6b, 0x9e, 0x73]); // LDLOC3, ADD, STLOC3
    script.extend_from_slice(&[0x6b, 0x40]); // LDLOC3, RET

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| match callback_api {
            api if api == get_network => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(860_833_102)],
                })
            }
            api if api == get_script_container => {
                assert!(stack.is_empty());
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![
                        StackValue::ByteString(vec![0x77; 32]),
                        StackValue::Integer(0),
                        StackValue::Integer(1),
                        StackValue::ByteString(vec![0x33; 20]),
                        StackValue::Integer(0),
                        StackValue::Integer(100_000_000),
                        StackValue::Integer(10),
                        StackValue::ByteString(vec![0xAA; 1024]),
                    ])],
                })
            }
            api if api == contract_call => {
                assert_eq!(stack.len(), 4);
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                })
            }
            other => panic!("unexpected syscall 0x{other:08x}"),
        },
    )
    .expect(
        "single contract-call continuation should survive after a full transaction-shaped script-container payload",
    );

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn consecutive_large_dynamic_calls_can_return_single_final_integer() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = Vec::new();
    for _ in 0..2 {
        script.push(0x0d);
        script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
        script.extend_from_slice(&gt);
        script.push(0x11);
        script.push(0xc0);
        script.push(0x1f);
        script.push(0x0c);
        script.push(19);
        script.extend_from_slice(b"bls12381Deserialize");
        script.push(0x0c);
        script.push(20);
        script.extend_from_slice(&[0x55; 20]);
        script.push(0x41);
        script.extend_from_slice(&api.to_le_bytes());
    }
    script.push(0x40);

    let mut call_count = 0u32;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            call_count += 1;
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(call_count as i64)],
            })
        },
    )
    .expect("two consecutive large dynamic calls should not trap");

    assert_eq!(call_count, 2);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
}

#[test]
fn custom_host_callback_handles_interop_array_argument() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);
    let expected = vec![StackValue::Array(vec![
        StackValue::Interop(11),
        StackValue::Interop(29),
    ])];
    let mut observed = None;

    let result = execute_script_with_host_and_stack(
        &script,
        expected.clone(),
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed = Some(stack.to_vec());
            Ok(HostCallbackResult { stack: Vec::new() })
        },
    )
    .expect("host runtime should pass interop arrays through the guest boundary");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
    assert_eq!(observed, Some(expected));
}

#[test]
fn custom_host_callback_handles_packed_interop_results_across_multiple_syscalls() {
    let deserialize_api = neo_riscv_abi::interop_hash("Crypto.Deserialize");
    let aggregate_api = neo_riscv_abi::interop_hash("Crypto.Aggregate");
    let script = vec![
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x12,
        0xc0,
        0x41,
        aggregate_api.to_le_bytes()[0],
        aggregate_api.to_le_bytes()[1],
        aggregate_api.to_le_bytes()[2],
        aggregate_api.to_le_bytes()[3],
        0x40,
    ];
    let mut deserialize_count = 0;
    let mut observed_aggregate = None;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api == deserialize_api {
                deserialize_count += 1;
                let mut next_stack = stack.to_vec();
                next_stack.push(StackValue::Interop(deserialize_count));
                return Ok(HostCallbackResult { stack: next_stack });
            }

            if callback_api != aggregate_api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed_aggregate = Some(stack.to_vec());
            Ok(HostCallbackResult {
                stack: vec![StackValue::Interop(99)],
            })
        },
    )
    .expect("host runtime should preserve packed interop results across multiple syscalls");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Interop(99)]);
    assert_eq!(
        observed_aggregate,
        Some(vec![StackValue::Array(vec![
            StackValue::Interop(2),
            StackValue::Interop(1)
        ])])
    );
}

#[test]
fn custom_host_callback_preserves_interop_result_between_syscalls() {
    let deserialize_api = neo_riscv_abi::interop_hash("Crypto.Deserialize");
    let script = vec![
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_second = None;
    let mut deserialize_count = 0u64;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            assert_eq!(api, deserialize_api);
            deserialize_count += 1;
            if deserialize_count == 2 {
                observed_second = Some(stack.to_vec());
            }
            let mut next_stack = stack.to_vec();
            next_stack.push(StackValue::Interop(deserialize_count));
            Ok(HostCallbackResult { stack: next_stack })
        },
    )
    .expect("interop results should survive into the next syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(observed_second, Some(vec![StackValue::Interop(1)]));
    assert_eq!(
        result.stack,
        vec![StackValue::Interop(1), StackValue::Interop(2)]
    );
}

#[test]
fn custom_host_callback_preserves_integer_result_between_syscalls() {
    let deserialize_api = neo_riscv_abi::interop_hash("Crypto.Deserialize");
    let script = vec![
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x41,
        deserialize_api.to_le_bytes()[0],
        deserialize_api.to_le_bytes()[1],
        deserialize_api.to_le_bytes()[2],
        deserialize_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_second = None;
    let mut call_count = 0i64;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            assert_eq!(api, deserialize_api);
            call_count += 1;
            if call_count == 2 {
                observed_second = Some(stack.to_vec());
            }
            let mut next_stack = stack.to_vec();
            next_stack.push(StackValue::Integer(call_count));
            Ok(HostCallbackResult { stack: next_stack })
        },
    )
    .expect("integer results should survive into the next syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(observed_second, Some(vec![StackValue::Integer(1)]));
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
}

#[test]
fn custom_host_callback_preserves_integer_then_bytestring_between_syscalls() {
    let api = neo_riscv_abi::interop_hash("System.Test.Mixed");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let mut observed_second = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(8)],
                }),
                2 => {
                    observed_second = Some(stack.to_vec());
                    Ok(HostCallbackResult {
                        stack: vec![
                            StackValue::Integer(8),
                            StackValue::ByteString(b"GAS".to_vec()),
                        ],
                    })
                }
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("mixed integer/bytestring results should survive into the second callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(observed_second, Some(vec![StackValue::Integer(8)]));
    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(8),
            StackValue::ByteString(b"GAS".to_vec())
        ]
    );
}

#[test]
fn local_storage_round_trip_survives_delete_and_following_get() {
    let local_put = neo_riscv_abi::interop_hash("System.Storage.Local.Put");
    let local_get = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let local_delete = neo_riscv_abi::interop_hash("System.Storage.Local.Delete");
    let script = vec![
        0x0c,
        0x01,
        b'k',
        0x0c,
        0x01,
        b'v',
        0x41,
        local_put.to_le_bytes()[0],
        local_put.to_le_bytes()[1],
        local_put.to_le_bytes()[2],
        local_put.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get.to_le_bytes()[0],
        local_get.to_le_bytes()[1],
        local_get.to_le_bytes()[2],
        local_get.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_delete.to_le_bytes()[0],
        local_delete.to_le_bytes()[1],
        local_delete.to_le_bytes()[2],
        local_delete.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get.to_le_bytes()[0],
        local_get.to_le_bytes()[1],
        local_get.to_le_bytes()[2],
        local_get.to_le_bytes()[3],
        0x40,
    ];

    let mut storage = std::collections::HashMap::<Vec<u8>, Vec<u8>>::new();
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| match api {
            value if value == local_put => {
                assert_eq!(stack.len(), 2);
                let key = match &stack[0] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put key bytes, got {other:?}"),
                };
                let val = match &stack[1] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put value bytes, got {other:?}"),
                };
                assert_eq!(key, b"k".to_vec());
                assert_eq!(val, b"v".to_vec());
                storage.insert(key, val);
                Ok(HostCallbackResult { stack: vec![] })
            }
            value if value == local_get => {
                assert!(!stack.is_empty());
                let key = match stack.last().expect("local-get key") {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-get key bytes, got {other:?}"),
                };
                assert_eq!(key, b"k".to_vec());
                let item = storage
                    .get(&key)
                    .cloned()
                    .map(StackValue::ByteString)
                    .unwrap_or(StackValue::Null);
                Ok(HostCallbackResult { stack: vec![item] })
            }
            value if value == local_delete => {
                assert!(!stack.is_empty());
                let key = match stack.last().expect("local-delete key") {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-delete key bytes, got {other:?}"),
                };
                assert_eq!(key, b"k".to_vec());
                storage.remove(&key);
                Ok(HostCallbackResult { stack: vec![] })
            }
            other => Err(format!("unexpected syscall 0x{other:08x}")),
        },
    )
    .expect("local storage round-trip should preserve the first get across delete");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"v".to_vec()), StackValue::Null]
    );
}

#[test]
fn local_storage_get_result_survives_pushdata_before_delete() {
    let local_put = neo_riscv_abi::interop_hash("System.Storage.Local.Put");
    let local_get = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let notify_api = neo_riscv_abi::interop_hash("System.Runtime.Notify");
    let script = vec![
        0x0c,
        0x01,
        b'k',
        0x0c,
        0x01,
        b'v',
        0x41,
        local_put.to_le_bytes()[0],
        local_put.to_le_bytes()[1],
        local_put.to_le_bytes()[2],
        local_put.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get.to_le_bytes()[0],
        local_get.to_le_bytes()[1],
        local_get.to_le_bytes()[2],
        local_get.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        notify_api.to_le_bytes()[0],
        notify_api.to_le_bytes()[1],
        notify_api.to_le_bytes()[2],
        notify_api.to_le_bytes()[3],
        0x40,
    ];

    let mut storage = std::collections::HashMap::<Vec<u8>, Vec<u8>>::new();
    let mut observed_notify = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| match api {
            value if value == local_put => {
                assert_eq!(stack.len(), 2);
                let key = match &stack[0] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put key bytes, got {other:?}"),
                };
                let val = match &stack[1] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put value bytes, got {other:?}"),
                };
                storage.insert(key, val);
                Ok(HostCallbackResult { stack: vec![] })
            }
            value if value == local_get => {
                assert_eq!(stack, &[StackValue::ByteString(b"k".to_vec())]);
                Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                })
            }
            value if value == notify_api => {
                observed_notify = Some(stack.to_vec());
                Ok(HostCallbackResult { stack: vec![] })
            }
            other => Err(format!("unexpected syscall 0x{other:08x}")),
        },
    )
    .expect("local storage get result should survive a following PUSHDATA before delete");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_notify,
        Some(vec![
            StackValue::ByteString(b"v".to_vec()),
            StackValue::ByteString(b"k".to_vec()),
        ])
    );
    assert!(result.stack.is_empty());
}

#[test]
fn local_storage_get_result_survives_delete_before_next_get() {
    let local_put = neo_riscv_abi::interop_hash("System.Storage.Local.Put");
    let local_get = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let local_delete = neo_riscv_abi::interop_hash("System.Storage.Local.Delete");
    let notify_api = neo_riscv_abi::interop_hash("System.Runtime.Notify");
    let script = vec![
        0x0c,
        0x01,
        b'k',
        0x0c,
        0x01,
        b'v',
        0x41,
        local_put.to_le_bytes()[0],
        local_put.to_le_bytes()[1],
        local_put.to_le_bytes()[2],
        local_put.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get.to_le_bytes()[0],
        local_get.to_le_bytes()[1],
        local_get.to_le_bytes()[2],
        local_get.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_delete.to_le_bytes()[0],
        local_delete.to_le_bytes()[1],
        local_delete.to_le_bytes()[2],
        local_delete.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        notify_api.to_le_bytes()[0],
        notify_api.to_le_bytes()[1],
        notify_api.to_le_bytes()[2],
        notify_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_notify = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| match api {
            value if value == local_put => {
                assert_eq!(
                    stack,
                    &[
                        StackValue::ByteString(b"k".to_vec()),
                        StackValue::ByteString(b"v".to_vec()),
                    ]
                );
                Ok(HostCallbackResult { stack: vec![] })
            }
            value if value == local_get => {
                assert_eq!(stack, &[StackValue::ByteString(b"k".to_vec())]);
                Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                })
            }
            value if value == local_delete => {
                assert_eq!(stack, &[StackValue::ByteString(b"k".to_vec())]);
                Ok(HostCallbackResult { stack: vec![] })
            }
            value if value == notify_api => {
                observed_notify = Some(stack.to_vec());
                Ok(HostCallbackResult { stack: vec![] })
            }
            other => Err(format!("unexpected syscall 0x{other:08x}")),
        },
    )
    .expect("local storage get result should survive delete before the next get");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_notify,
        Some(vec![
            StackValue::ByteString(b"v".to_vec()),
            StackValue::ByteString(b"k".to_vec()),
        ])
    );
    assert!(result.stack.is_empty());
}

#[test]
fn bytestring_result_survives_pushdata_before_next_syscall() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let notify_api = neo_riscv_abi::interop_hash("System.Runtime.Notify");
    let script = vec![
        0x41,
        platform_api.to_le_bytes()[0],
        platform_api.to_le_bytes()[1],
        platform_api.to_le_bytes()[2],
        platform_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        notify_api.to_le_bytes()[0],
        notify_api.to_le_bytes()[1],
        notify_api.to_le_bytes()[2],
        notify_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_notify = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                });
            }
            if api == notify_api {
                observed_notify = Some(stack.to_vec());
                return Ok(HostCallbackResult { stack: vec![] });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect("bytestring result should survive a following PUSHDATA before the next syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_notify,
        Some(vec![
            StackValue::ByteString(b"v".to_vec()),
            StackValue::ByteString(b"k".to_vec()),
        ])
    );
    assert!(result.stack.is_empty());
}

#[test]
fn retained_bytestring_survives_one_arg_syscall_with_no_results() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let log_api = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let script = vec![
        0x41,
        platform_api.to_le_bytes()[0],
        platform_api.to_le_bytes()[1],
        platform_api.to_le_bytes()[2],
        platform_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        log_api.to_le_bytes()[0],
        log_api.to_le_bytes()[1],
        log_api.to_le_bytes()[2],
        log_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_log = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                });
            }
            if api == log_api {
                observed_log = Some(stack.to_vec());
                return Ok(HostCallbackResult { stack: vec![] });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect("retained bytestring should survive a one-arg syscall that returns no results");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_log,
        Some(vec![StackValue::ByteString(b"k".to_vec())])
    );
    assert_eq!(result.stack, vec![StackValue::ByteString(b"v".to_vec())]);
}

#[test]
fn retained_bytestring_survives_no_result_then_null_result_syscalls() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let log_api = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let local_get_api = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let script = vec![
        0x41,
        platform_api.to_le_bytes()[0],
        platform_api.to_le_bytes()[1],
        platform_api.to_le_bytes()[2],
        platform_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        log_api.to_le_bytes()[0],
        log_api.to_le_bytes()[1],
        log_api.to_le_bytes()[2],
        log_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get_api.to_le_bytes()[0],
        local_get_api.to_le_bytes()[1],
        local_get_api.to_le_bytes()[2],
        local_get_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_log = None;
    let mut observed_get = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                });
            }
            if api == log_api {
                observed_log = Some(stack.to_vec());
                return Ok(HostCallbackResult { stack: vec![] });
            }
            if api == local_get_api {
                observed_get = Some(stack.to_vec());
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect(
        "retained bytestring should survive a null-returning syscall after a no-result syscall",
    );

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_log,
        Some(vec![StackValue::ByteString(b"k".to_vec())])
    );
    assert_eq!(
        observed_get,
        Some(vec![StackValue::ByteString(b"k".to_vec())])
    );
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"v".to_vec()), StackValue::Null]
    );
}

#[test]
fn retained_bytestring_and_null_can_be_observed_before_ret() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let log_api = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let local_get_api = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let notify_api = neo_riscv_abi::interop_hash("System.Runtime.Notify");
    let script = vec![
        0x41,
        platform_api.to_le_bytes()[0],
        platform_api.to_le_bytes()[1],
        platform_api.to_le_bytes()[2],
        platform_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        log_api.to_le_bytes()[0],
        log_api.to_le_bytes()[1],
        log_api.to_le_bytes()[2],
        log_api.to_le_bytes()[3],
        0x0c,
        0x01,
        b'k',
        0x41,
        local_get_api.to_le_bytes()[0],
        local_get_api.to_le_bytes()[1],
        local_get_api.to_le_bytes()[2],
        local_get_api.to_le_bytes()[3],
        0x41,
        notify_api.to_le_bytes()[0],
        notify_api.to_le_bytes()[1],
        notify_api.to_le_bytes()[2],
        notify_api.to_le_bytes()[3],
        0x40,
    ];

    let mut observed_notify = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                });
            }
            if api == log_api {
                return Ok(HostCallbackResult { stack: vec![] });
            }
            if api == local_get_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            if api == notify_api {
                observed_notify = Some(stack.to_vec());
                return Ok(HostCallbackResult { stack: vec![] });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect("retained bytestring/null pair should be observable before RET");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_notify,
        Some(vec![
            StackValue::ByteString(b"v".to_vec()),
            StackValue::Null
        ])
    );
    assert!(result.stack.is_empty());
}

#[test]
fn helper_syscalls_preserve_multiple_arguments_and_order_in_host_runtime() {
    let local_get_api = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let check_witness_api = neo_riscv_abi::interop_hash("System.Runtime.CheckWitness");
    let script = vec![
        0x57,
        0x00,
        0x03, // INITSLOT 0 locals, 3 args
        0x34,
        0x09, // CALL helper at ip 3 -> target 12
        0x45, // DROP helper return value
        0x7a, // LDARG2
        0x79, // LDARG1
        0x78, // LDARG0
        0x13, // PUSH3
        0xc0, // PACK
        0x40, // RET
        0x0c,
        0x01,
        0xff, // helper: PUSHDATA1 0xff
        0xdb,
        0x30, // CONVERT Buffer
        0x41, // SYSCALL Storage.Local.Get
        local_get_api.to_le_bytes()[0],
        local_get_api.to_le_bytes()[1],
        local_get_api.to_le_bytes()[2],
        local_get_api.to_le_bytes()[3],
        0x41, // SYSCALL Runtime.CheckWitness
        check_witness_api.to_le_bytes()[0],
        check_witness_api.to_le_bytes()[1],
        check_witness_api.to_le_bytes()[2],
        check_witness_api.to_le_bytes()[3],
        0x40, // RET
    ];

    let nef = vec![0x50, 0x00, 0x83, 0x00, 0x14, 0x00, 0x00, 0x00];
    let manifest = b"{\"name\":\"Contract\",\"abi\":{\"methods\":[]}}".to_vec();
    let owner = vec![
        0x41, 0xca, 0x6a, 0xbc, 0x77, 0x1a, 0x8b, 0x40, 0x75, 0x7a, 0x1a, 0x1b, 0x31, 0xd0, 0xae,
        0x7f, 0x5c, 0xf1, 0xfe, 0xac,
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::Null,
            StackValue::ByteString(manifest.clone()),
            StackValue::ByteString(nef.clone()),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == local_get_api {
                assert_eq!(stack, &[StackValue::Buffer(vec![0xff])]);
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(owner.clone())],
                });
            }
            if api == check_witness_api {
                assert_eq!(stack, &[StackValue::ByteString(owner.clone())]);
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect("helper syscalls should preserve caller arguments in host runtime");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::ByteString(nef),
            StackValue::ByteString(manifest),
            StackValue::Null,
        ])]
    );
}

#[test]
fn helper_syscalls_preserve_large_arguments_before_callt_in_host_runtime() {
    let local_get_api = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
    let check_witness_api = neo_riscv_abi::interop_hash("System.Runtime.CheckWitness");
    let callt_update_api = neo_riscv_guest::CALLT_MARKER;
    let script = vec![
        0x57,
        0x00,
        0x03, // INITSLOT 0 locals, 3 args
        0x34,
        0x0c, // CALL helper at ip 3 -> target 15
        0x24,
        0x03, // JMPIF authorized path at ip 5 -> target 8
        0x40, // RET (unauthorized)
        0x7a, // LDARG2
        0x79, // LDARG1
        0x78, // LDARG0
        0x37,
        0x00,
        0x00, // CALLT 0
        0x40, // RET
        0x0c,
        0x01,
        0xff, // helper: PUSHDATA1 0xff
        0xdb,
        0x30, // CONVERT Buffer
        0x41,
        local_get_api.to_le_bytes()[0],
        local_get_api.to_le_bytes()[1],
        local_get_api.to_le_bytes()[2],
        local_get_api.to_le_bytes()[3],
        0x41,
        check_witness_api.to_le_bytes()[0],
        check_witness_api.to_le_bytes()[1],
        check_witness_api.to_le_bytes()[2],
        check_witness_api.to_le_bytes()[3],
        0x40,
    ];

    let nef = vec![0x50, 0x00, 0xa3, 0x00, 0x14, 0x00, 0x00, 0x00];
    let manifest = vec![b'm'; 415];
    let owner = vec![
        0x41, 0xca, 0x6a, 0xbc, 0x77, 0x1a, 0x8b, 0x40, 0x75, 0x7a, 0x1a, 0x1b, 0x31, 0xd0, 0xae,
        0x7f, 0x5c, 0xf1, 0xfe, 0xac,
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::Null,
            StackValue::ByteString(manifest.clone()),
            StackValue::ByteString(nef.clone()),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == local_get_api {
                assert_eq!(stack, &[StackValue::Buffer(vec![0xff])]);
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(owner.clone())],
                });
            }
            if api == check_witness_api {
                assert_eq!(stack, &[StackValue::ByteString(owner.clone())]);
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }
            if api == callt_update_api {
                assert_eq!(
                    stack,
                    &[
                        StackValue::Null,
                        StackValue::ByteString(manifest.clone()),
                        StackValue::ByteString(nef.clone()),
                    ]
                );
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Null],
                });
            }
            Err(format!("unexpected syscall 0x{api:08x}"))
        },
    )
    .expect("helper syscalls should preserve large arguments before CALLT in host runtime");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Null]);
}

#[test]
fn custom_host_callback_can_return_two_integers() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Integer(1), StackValue::Integer(2)],
            })
        },
    )
    .expect("two-item callback results should round-trip correctly");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
}

#[test]
fn custom_host_callback_preserves_initial_two_integer_response_between_syscalls() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let mut observed_second = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1), StackValue::Integer(2)],
                }),
                2 => {
                    observed_second = Some(stack.to_vec());
                    Ok(HostCallbackResult {
                        stack: stack.to_vec(),
                    })
                }
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("two-item first response should survive into the second callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_second,
        Some(vec![StackValue::Integer(1), StackValue::Integer(2)])
    );
}

#[test]
fn custom_host_callback_state_persists_across_multiple_syscalls() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            Ok(HostCallbackResult {
                stack: stack.to_vec(),
            })
        },
    )
    .expect("callback state should persist across syscalls");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(call_count, 2);
}

#[test]
fn custom_host_callback_can_return_fresh_two_item_second_response() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let mut second_count: Option<i64> = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1)],
                }),
                2 => {
                    second_count = Some(call_count);
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Integer(2), StackValue::Integer(3)],
                    })
                }
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("fresh two-item second response should survive");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(second_count, Some(2));
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(2), StackValue::Integer(3)]
    );
}

#[test]
fn debug_success_trace_for_two_item_second_response() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let trace = debug_execute_script_with_host_and_stack(
        &script,
        Vec::new(),
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1)],
                }),
                2 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(2), StackValue::Integer(3)],
                }),
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("debug helper should complete")
    .1;

    assert_eq!(
        trace,
        Some((
            23,
            vec![0, 2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0,],
        ))
    );
}

#[test]
fn custom_host_callback_exposes_second_response_to_next_syscall() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let mut observed_third = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1)],
                }),
                2 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1), StackValue::Integer(2)],
                }),
                3 => {
                    observed_third = Some(stack.to_vec());
                    Ok(HostCallbackResult {
                        stack: stack.to_vec(),
                    })
                }
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("third callback should observe the second callback response");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_third,
        Some(vec![StackValue::Integer(1), StackValue::Integer(2)])
    );
}

#[test]
fn custom_host_callback_exposes_fresh_second_response_to_next_syscall() {
    let api = neo_riscv_abi::interop_hash("System.Test.Multi");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut call_count = 0i64;
    let mut observed_third = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            call_count += 1;
            match call_count {
                1 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1)],
                }),
                2 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(1), StackValue::Integer(2)],
                }),
                3 => {
                    observed_third = Some(_stack.to_vec());
                    Ok(HostCallbackResult {
                        stack: _stack.to_vec(),
                    })
                }
                _ => unreachable!("unexpected extra callback"),
            }
        },
    )
    .expect("third callback should observe a fresh second response");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        observed_third,
        Some(vec![StackValue::Integer(1), StackValue::Integer(2)])
    );
}

#[test]
fn custom_host_callback_handles_contract_call_shape_with_interop_array_argument() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);
    let expected = vec![
        StackValue::Array(vec![StackValue::Interop(2), StackValue::Interop(1)]),
        StackValue::Integer(i64::from(0x0f_u8)),
        StackValue::ByteString(b"bls12381Add".to_vec()),
        StackValue::ByteString(vec![0x55; 20]),
    ];
    let mut observed = None;

    let result = execute_script_with_host_and_stack(
        &script,
        expected.clone(),
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, stack| {
            if callback_api != api {
                return Err(format!("unexpected syscall 0x{callback_api:08x}"));
            }

            observed = Some(stack.to_vec());
            Ok(HostCallbackResult {
                stack: vec![StackValue::Interop(77)],
            })
        },
    )
    .expect("host runtime should pass System.Contract.Call interop-array arguments through the guest boundary");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Interop(77)]);
    assert_eq!(observed, Some(expected));
}

#[test]
fn unsupported_opcode_returns_fault_state() {
    let error = execute_script(&[0xff]).expect_err("unsupported opcode should fault");
    assert!(
        error.contains("unsupported opcode") || error.contains("Failed to decode result"),
        "error should mention unsupported opcode: {error}"
    );
}

#[test]
fn and_on_booleans_through_polkavm_returns_integer() {
    // PUSH0 NOT (→ true), PUSH0 NOT (→ true), AND → Integer(1)
    // NeoVM AND on booleans returns Integer, not Boolean
    // NOT = 0xaa, AND = 0x91
    let result = execute_script(&[0x10, 0xaa, 0x10, 0xaa, 0x91, 0x40])
        .expect("PolkaVM path should handle boolean AND");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn handles_multiple_syscalls_with_pack_and_interop() {
    // Mimics BLS12-381 flow: 2 SYSCALLs that return Interop handles, PACK them, 3rd SYSCALL
    let syscall1 = neo_riscv_abi::interop_hash("System.Test.Deserialize");
    let syscall2 = neo_riscv_abi::interop_hash("System.Test.Add");

    let mut script = vec![];
    // First SYSCALL: push "test", SYSCALL
    script.push(0x0c); // PUSHDATA1
    script.push(4); // length
    script.extend_from_slice(b"test");
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&syscall1.to_le_bytes());
    // Second SYSCALL: push "test", SYSCALL
    script.push(0x0c); // PUSHDATA1
    script.push(4); // length
    script.extend_from_slice(b"test");
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&syscall1.to_le_bytes());
    // PACK
    script.push(0x12); // PUSH2
    script.push(0xc0); // PACK
                       // Third SYSCALL: push "add", SYSCALL
    script.push(0x0c); // PUSHDATA1
    script.push(3); // length
    script.extend_from_slice(b"add");
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&syscall2.to_le_bytes());
    script.push(0x40); // RET

    let mut call_count = 0u32;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            call_count += 1;
            if api == syscall1 {
                // Pop the consumed "test" argument, push the Interop result
                let mut new_stack = stack.to_vec();
                new_stack.pop(); // pop "test"
                new_stack.push(StackValue::Interop(call_count as u64));
                Ok(HostCallbackResult { stack: new_stack })
            } else if api == syscall2 {
                // Simulates bls12381Add: pop "add" method name AND the packed Array argument
                // In real NeoVM, SYSCALL pops all its arguments from the stack
                let mut new_stack = stack.to_vec();
                new_stack.pop(); // pop "add"
                new_stack.pop(); // pop the packed Array
                new_stack.push(StackValue::Integer(42));
                Ok(HostCallbackResult { stack: new_stack })
            } else {
                Err(format!("unknown syscall 0x{api:08x}"))
            }
        },
    )
    .expect("PolkaVM should handle multiple syscalls with pack and interop");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn large_dynamic_call_with_bytestring_argument_can_return_interop() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let gt = vec![0xaa; 576];
    let mut script = Vec::new();
    script.push(0x0d);
    script.extend_from_slice(&(gt.len() as u16).to_le_bytes());
    script.extend_from_slice(&gt);
    script.push(0x11);
    script.push(0xc0);
    script.push(0x1f);
    script.push(0x0c);
    script.push(19);
    script.extend_from_slice(b"bls12381Deserialize");
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41);
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |callback_api, _ip, _context, _stack| {
            assert_eq!(callback_api, api);
            Ok(HostCallbackResult {
                stack: vec![StackValue::Interop(1)],
            })
        },
    )
    .expect("large dynamic calls should round-trip a callback result");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Interop(1)]);
}

unsafe extern "C" fn ffi_error_callback(
    _user_data: *mut c_void,
    _api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let payload = b"ffi callback failure".to_vec().into_boxed_slice();
    let payload_len = payload.len();
    let payload_ptr = Box::into_raw(payload) as *mut u8;
    unsafe {
        *output = NativeHostResult {
            stack_ptr: ptr::null_mut(),
            stack_len: 0,
            error_ptr: payload_ptr,
            error_len: payload_len,
        };
    }
    true
}

unsafe extern "C" fn ffi_error_free_callback(
    _user_data: *mut c_void,
    result: *mut NativeHostResult,
) {
    unsafe {
        if result.is_null() || (*result).error_ptr.is_null() || (*result).error_len == 0 {
            return;
        }
        let bytes = ptr::slice_from_raw_parts_mut((*result).error_ptr, (*result).error_len);
        drop(Box::from_raw(bytes));
        (*result).error_ptr = ptr::null_mut();
        (*result).error_len = 0;
    }
}

#[repr(C)]
struct FfiMixedState {
    call_count: u32,
}

struct FfiStorageContextState {
    calls: Vec<(u32, Vec<StackValue>)>,
    token: Vec<u8>,
}

struct FfiOracleSuccessState {
    stored: Option<Vec<StackValue>>,
}

struct FfiAttributeState {
    observed_checkwitness: Option<Vec<StackValue>>,
}

unsafe extern "C" fn ffi_storage_context_callback(
    user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let state = unsafe { &mut *(user_data as *mut FfiStorageContextState) };
    let stack = if input_stack_ptr.is_null() || input_stack_len == 0 {
        Vec::new()
    } else {
        match unsafe { copy_test_native_stack_items(input_stack_ptr.cast_mut(), input_stack_len) } {
            Ok(stack) => stack,
            Err(error) => {
                let payload = error.into_bytes().into_boxed_slice();
                let error_len = payload.len();
                let error_ptr = Box::into_raw(payload) as *mut u8;
                unsafe {
                    *output = NativeHostResult {
                        stack_ptr: ptr::null_mut(),
                        stack_len: 0,
                        error_ptr,
                        error_len,
                    };
                }
                return true;
            }
        }
    };
    state.calls.push((api, stack));

    let result_stack = match api {
        api if api == neo_riscv_abi::interop_hash("System.Storage.GetContext") => {
            vec![StackValue::ByteString(state.token.clone())]
        }
        api if api == neo_riscv_abi::interop_hash("System.Storage.Put") => Vec::new(),
        api if api == neo_riscv_abi::interop_hash("System.Storage.Get") => {
            vec![StackValue::ByteString(b"v".to_vec())]
        }
        _ => {
            let payload = format!("unexpected syscall 0x{api:08x}")
                .into_bytes()
                .into_boxed_slice();
            let error_len = payload.len();
            let error_ptr = Box::into_raw(payload) as *mut u8;
            unsafe {
                *output = NativeHostResult {
                    stack_ptr: ptr::null_mut(),
                    stack_len: 0,
                    error_ptr,
                    error_len,
                };
            }
            return true;
        }
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&result_stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_oracle_success_callback(
    user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let state = unsafe { &mut *(user_data as *mut FfiOracleSuccessState) };
    let hash_api = neo_riscv_abi::interop_hash("System.Runtime.GetCallingScriptHash");
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let put = neo_riscv_abi::interop_hash("System.Storage.Put");

    let stack = unsafe {
        copy_test_native_stack_items(
            input_stack_ptr as *mut neo_riscv_host::NativeStackItem,
            input_stack_len,
        )
    }
    .unwrap_or_default();

    let response_stack = if api == hash_api {
        vec![StackValue::ByteString(vec![
            0x58, 0x87, 0x17, 0x11, 0x7e, 0x0a, 0xa8, 0x10, 0x72, 0xaf, 0xab, 0x71, 0xd2, 0xdd,
            0x89, 0xfe, 0x7c, 0x4b, 0x92, 0xfe,
        ])]
    } else if api == (neo_riscv_guest::CALLT_MARKER | 2) {
        vec![StackValue::Array(vec![StackValue::ByteString(
            b"Hello World!".to_vec(),
        )])]
    } else if api == get_context {
        vec![StackValue::ByteString(storage_context_token(0, false))]
    } else if api == put {
        state.stored = Some(stack);
        Vec::new()
    } else {
        return false;
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&response_stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_attribute_callback(
    user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let state = unsafe { &mut *(user_data as *mut FfiAttributeState) };
    let stack = unsafe {
        copy_test_native_stack_items(
            input_stack_ptr as *mut neo_riscv_host::NativeStackItem,
            input_stack_len,
        )
    }
    .unwrap_or_default();

    let response_stack = if api == neo_riscv_guest::CALLT_MARKER {
        vec![
            StackValue::Array(vec![StackValue::Null]),
            StackValue::ByteString(vec![0; 20]),
        ]
    } else if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
        state.observed_checkwitness = Some(stack);
        vec![StackValue::Boolean(true)]
    } else {
        return false;
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&response_stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_mixed_callback(
    user_data: *mut c_void,
    _api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let state = unsafe { &mut *(user_data as *mut FfiMixedState) };
    state.call_count += 1;

    let stack = match state.call_count {
        1 => vec![StackValue::Integer(8)],
        2 => vec![
            StackValue::Integer(8),
            StackValue::ByteString(b"GAS".to_vec()),
        ],
        _ => return false,
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_mixed_free_callback(
    _user_data: *mut c_void,
    result: *mut NativeHostResult,
) {
    if result.is_null() {
        return;
    }
    let result = unsafe { &mut *result };
    if !result.stack_ptr.is_null() {
        unsafe { free_native_stack_items(result.stack_ptr, result.stack_len) };
        result.stack_ptr = ptr::null_mut();
        result.stack_len = 0;
    }
}

unsafe extern "C" fn ffi_callt_array_callback(
    _user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    if api != neo_riscv_guest::CALLT_MARKER {
        return false;
    }

    let stack = vec![StackValue::Array(vec![StackValue::ByteString(
        b"Hello World!".to_vec(),
    )])];
    let (stack_ptr, stack_len) = build_native_stack_items(&stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_callt_null_helper_callback(
    _user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let runtime_log = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let stack = match api {
        api if api == (neo_riscv_guest::CALLT_MARKER | 2) => vec![StackValue::Null],
        api if api == runtime_log => Vec::new(),
        _ => return false,
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_callt_block_helper_callback(
    _user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    if api != (neo_riscv_guest::CALLT_MARKER | 2) {
        return false;
    }

    let block_like = StackValue::Struct(vec![
        StackValue::ByteString(vec![0xd9; 32]),
        StackValue::Integer(0),
        StackValue::ByteString(vec![
            0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c,
            0x7d, 0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4,
            0xef, 0x1d, 0x4d, 0x1f,
        ]),
        StackValue::ByteString(vec![0x72; 32]),
        StackValue::Integer(1),
        StackValue::Integer(2),
        StackValue::Integer(3),
        StackValue::Integer(4),
        StackValue::ByteString(vec![0x6b; 20]),
        StackValue::Integer(1),
    ]);

    let (stack_ptr, stack_len) = build_native_stack_items(&[block_like]);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_callt_signers_callback(
    _user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    if api != (neo_riscv_guest::CALLT_MARKER | 4) {
        return false;
    }

    let signers = StackValue::Array(vec![StackValue::Array(vec![
        StackValue::ByteString(vec![0x22; 20]),
        StackValue::Integer(0x80),
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
        StackValue::Array(vec![]),
    ])]);

    let (stack_ptr, stack_len) = build_native_stack_items(&[signers]);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_callt_transaction_then_signers_callback(
    _user_data: *mut c_void,
    api: u32,
    _instruction_pointer: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let stack = match api {
        value if value == (neo_riscv_guest::CALLT_MARKER | 3) => StackValue::Struct(vec![
            StackValue::ByteString(vec![
                0xd9, 0xe0, 0xe7, 0xe0, 0x1e, 0xe5, 0x5d, 0x33, 0xee, 0x14, 0xc0, 0xda, 0x41, 0xfa,
                0xe5, 0x2a, 0x8a, 0xd4, 0x53, 0xfd, 0x6e, 0xdb, 0xdb, 0xc1, 0x47, 0x60, 0xd7, 0x4c,
                0xf1, 0xc1, 0xa1, 0xd4,
            ]),
            StackValue::Integer(0),
            StackValue::Integer(0x01020304),
            StackValue::ByteString(vec![0x11; 20]),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::ByteString(vec![0x40]),
        ]),
        value if value == (neo_riscv_guest::CALLT_MARKER | 4) => {
            StackValue::Array(vec![StackValue::Array(vec![
                StackValue::ByteString(vec![0x22; 20]),
                StackValue::Integer(0x80),
                StackValue::Array(vec![]),
                StackValue::Array(vec![]),
                StackValue::Array(vec![]),
            ])])
        }
        _ => return false,
    };

    let (stack_ptr, stack_len) = build_native_stack_items(&[stack]);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

#[test]
fn ffi_host_callback_errors_fault_without_trapping() {
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            ptr::null(),
            0,
            ptr::null_mut(),
            ffi_error_callback,
            ffi_error_free_callback,
            &mut output,
        )
    };

    let error = if output.error_ptr.is_null() || output.error_len == 0 {
        String::new()
    } else {
        unsafe {
            String::from_utf8_lossy(slice::from_raw_parts(output.error_ptr, output.error_len))
                .into_owned()
        }
    };

    assert!(invoked, "native execution ABI should be invoked");
    assert_eq!(output.state, 1, "ffi callback errors should fault the VM");
    assert!(
        error.contains("ffi callback failure"),
        "ffi callback errors should be surfaced directly, got: {error}"
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_callt_null_result_can_flow_through_two_arg_helper() {
    let runtime_log = neo_riscv_abi::interop_hash("System.Runtime.Log");
    let script = vec![
        0x57,
        0x01,
        0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37,
        0x02,
        0x00, // CALLT 2
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34,
        0x03, // CALL +3 -> helper at ip=13
        0x40, // RET
        0x57,
        0x00,
        0x02, // helper: INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0xd8, // ISNULL
        0x26,
        0x15, // JMPIFNOT +21 -> else branch at ip=39
        0x0c,
        0x0a,
        b'N',
        b'U',
        b'L',
        b'L',
        b' ',
        b'B',
        b'l',
        b'o',
        b'c',
        b'k',
        0x41,
        (runtime_log & 0xff) as u8,
        ((runtime_log >> 8) & 0xff) as u8,
        ((runtime_log >> 16) & 0xff) as u8,
        ((runtime_log >> 24) & 0xff) as u8,
        0x0b, // PUSHNULL
        0x40, // RET
        0x08, // else: PUSHT
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(Vec::new()),
        StackValue::ByteString(vec![0x01; 32]),
    ];
    let (initial_ptr, initial_len) = build_native_stack_items(&initial_stack);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            initial_ptr,
            initial_len,
            ptr::null_mut(),
            ffi_callt_null_helper_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe { free_native_stack_items(initial_ptr, initial_len) };
    assert!(invoked, "ffi execution should be invoked");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi output stack should decode");
    assert_eq!(output.state, 0, "ffi helper flow should HALT");
    assert_eq!(stack, vec![StackValue::Null]);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_callt_block_like_struct_survives_local_and_helper_pickitem() {
    let prev_hash = vec![
        0x15, 0x7c, 0xa8, 0xda, 0x91, 0xa2, 0x99, 0x58, 0x6f, 0x5f, 0xaa, 0xc4, 0x26, 0x7c, 0x7d,
        0x77, 0xec, 0x6b, 0xa0, 0x79, 0x3f, 0x8d, 0x9b, 0x7b, 0x5e, 0xaa, 0x6f, 0xa4, 0xef, 0x1d,
        0x4d, 0x1f,
    ];
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37, 0x02, 0x00, // CALLT 2
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34, 0x03, // CALL +3 -> helper
        0x40, // RET
        0x57, 0x00, 0x02, // helper
        0x78, // LDARG0
        0x12, // PUSH2
        0xCE, // PICKITEM
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"PrevHash".to_vec()),
        StackValue::ByteString(vec![0x01; 32]),
    ];
    let (initial_ptr, initial_len) = build_native_stack_items(&initial_stack);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            initial_ptr,
            initial_len,
            ptr::null_mut(),
            ffi_callt_block_helper_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe { free_native_stack_items(initial_ptr, initial_len) };
    assert!(invoked, "ffi execution should be invoked");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi output stack should decode");
    assert_eq!(output.state, 0, "ffi block helper flow should HALT");
    assert_eq!(stack, vec![StackValue::ByteString(prev_hash)]);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_tx_like_struct_hash_then_callt_signers() {
    let script = vec![
        0x57, 0x00, 0x02, // INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0x10, // PUSH0
        0xCE, // PICKITEM -> tx.Hash
        0x37, 0x04, 0x00, // CALLT 4 -> getTransactionSigners
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"Signers".to_vec()),
        StackValue::Struct(vec![
            StackValue::ByteString(vec![
                0xd9, 0xe0, 0xe7, 0xe0, 0x1e, 0xe5, 0x5d, 0x33, 0xee, 0x14, 0xc0, 0xda, 0x41, 0xfa,
                0xe5, 0x2a, 0x8a, 0xd4, 0x53, 0xfd, 0x6e, 0xdb, 0xdb, 0xc1, 0x47, 0x60, 0xd7, 0x4c,
                0xf1, 0xc1, 0xa1, 0xd4,
            ]),
            StackValue::Integer(0),
            StackValue::Integer(0x01020304),
            StackValue::ByteString(vec![0x11; 20]),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::ByteString(vec![0x40]),
        ]),
    ];
    let (initial_ptr, initial_len) = build_native_stack_items(&initial_stack);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            initial_ptr,
            initial_len,
            ptr::null_mut(),
            ffi_callt_signers_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe { free_native_stack_items(initial_ptr, initial_len) };
    assert!(invoked, "ffi execution should be invoked");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi output stack should decode");
    assert_eq!(output.state, 0, "ffi tx/signers flow should HALT");
    assert_eq!(
        stack,
        vec![StackValue::Array(vec![StackValue::Array(vec![
            StackValue::ByteString(vec![0x22; 20]),
            StackValue::Integer(0x80),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
        ])])]
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_callt_transaction_helper_then_callt_signers_with_live_args() {
    let tx_hash = vec![
        0xd9, 0xe0, 0xe7, 0xe0, 0x1e, 0xe5, 0x5d, 0x33, 0xee, 0x14, 0xc0, 0xda, 0x41, 0xfa, 0xe5,
        0x2a, 0x8a, 0xd4, 0x53, 0xfd, 0x6e, 0xdb, 0xdb, 0xc1, 0x47, 0x60, 0xd7, 0x4c, 0xf1, 0xc1,
        0xa1, 0xd4,
    ];
    let script = vec![
        0x57, 0x01, 0x02, // INITSLOT 1 local, 2 args
        0x78, // LDARG0
        0x37, 0x03, 0x00, // CALLT 3 -> getTransaction
        0x70, // STLOC0
        0x79, // LDARG1
        0x68, // LDLOC0
        0x34, 0x06, // CALL +6 -> helper
        0x37, 0x04, 0x00, // CALLT 4 -> getTransactionSigners
        0x40, // RET
        0x57, 0x00, 0x02, // helper
        0x78, // LDARG0
        0x10, // PUSH0
        0xCE, // PICKITEM -> tx.Hash
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"Signers".to_vec()),
        StackValue::ByteString(tx_hash),
    ];
    let (initial_ptr, initial_len) = build_native_stack_items(&initial_stack);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            initial_ptr,
            initial_len,
            ptr::null_mut(),
            ffi_callt_transaction_then_signers_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe { free_native_stack_items(initial_ptr, initial_len) };
    assert!(invoked, "ffi execution should be invoked");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi output stack should decode");
    assert_eq!(output.state, 0, "ffi tx helper/signers flow should HALT");
    assert_eq!(
        stack,
        vec![StackValue::Array(vec![StackValue::Array(vec![
            StackValue::ByteString(vec![0x22; 20]),
            StackValue::Integer(0x80),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
            StackValue::Array(vec![]),
        ])])]
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_large_dynamic_call_host_error_surfaces_without_trapping() {
    let syscall = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let initial_stack = vec![
        StackValue::Array(vec![StackValue::ByteString(vec![0x42; 65_536])]),
        StackValue::Integer(i64::from(0x0f_u8)),
        StackValue::ByteString(b"deploy".to_vec()),
        StackValue::ByteString(vec![0x55; 20]),
    ];
    let (initial_ptr, initial_len) = build_native_stack_items(&initial_stack);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            initial_ptr,
            initial_len,
            ptr::null_mut(),
            ffi_error_callback,
            ffi_error_free_callback,
            &mut output,
        )
    };

    let error = if output.error_ptr.is_null() || output.error_len == 0 {
        String::new()
    } else {
        unsafe {
            String::from_utf8_lossy(slice::from_raw_parts(output.error_ptr, output.error_len))
                .into_owned()
        }
    };

    assert!(invoked, "native execution ABI should be invoked");
    assert_eq!(output.state, 1, "ffi callback errors should fault the VM");
    assert!(
        error.contains("ffi callback failure"),
        "ffi callback errors should be surfaced directly, got: {error}"
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
        free_native_stack_items(initial_ptr, initial_len);
    }
}

#[test]
fn ffi_large_dynamic_call_wrapper_host_error_surfaces_without_trapping() {
    let api = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = Vec::new();
    script.push(0x0b); // PUSHNULL
    script.push(0x0e); // PUSHDATA4
    script.extend_from_slice(&(65_536u32).to_le_bytes());
    script.extend_from_slice(&vec![0x42; 65_536]);
    script.push(0x0c); // PUSHDATA1
    script.push(1);
    script.push(0xaa);
    script.push(0x13); // PUSH3
    script.push(0xc0); // PACK
    script.push(0x1f); // PUSH15 (CallFlags.All)
    script.push(0x0c); // PUSHDATA1
    script.push(6);
    script.extend_from_slice(b"deploy");
    script.push(0x0c); // PUSHDATA1
    script.push(20);
    script.extend_from_slice(&[0x55; 20]);
    script.push(0x41); // SYSCALL
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40); // RET

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            20_000_000_000,
            300_000,
            ptr::null(),
            0,
            ptr::null_mut(),
            ffi_error_callback,
            ffi_error_free_callback,
            &mut output,
        )
    };

    let error = if output.error_ptr.is_null() || output.error_len == 0 {
        String::new()
    } else {
        unsafe {
            String::from_utf8_lossy(slice::from_raw_parts(output.error_ptr, output.error_len))
                .into_owned()
        }
    };

    assert!(invoked, "native execution ABI should be invoked");
    assert_eq!(output.state, 1, "ffi callback errors should fault the VM");
    assert!(
        error.contains("ffi callback failure"),
        "ffi wrapped host errors should be surfaced directly, got: {error}"
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_mixed_integer_and_bytestring_results_round_trip() {
    let api = neo_riscv_abi::interop_hash("System.Test.Mixed");
    let script = vec![
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x41,
        api.to_le_bytes()[0],
        api.to_le_bytes()[1],
        api.to_le_bytes()[2],
        api.to_le_bytes()[3],
        0x40,
    ];

    let mut state = Box::new(FfiMixedState { call_count: 0 });
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            ptr::null(),
            0,
            (&mut *state) as *mut FfiMixedState as *mut c_void,
            ffi_mixed_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi stack should decode");
    assert_eq!(
        stack,
        vec![
            StackValue::Integer(8),
            StackValue::ByteString(b"GAS".to_vec())
        ]
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn storage_context_token_round_trips_across_syscalls_in_host_path() {
    let script = build_storage_context_round_trip_script();
    let token = storage_context_token(0x1234_5678, false);
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let put = neo_riscv_abi::interop_hash("System.Storage.Put");
    let get = neo_riscv_abi::interop_hash("System.Storage.Get");
    let mut calls = Vec::new();

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            calls.push((api, stack.to_vec()));
            match api {
                api if api == get_context => Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(token.clone())],
                }),
                api if api == put => Ok(HostCallbackResult { stack: Vec::new() }),
                api if api == get => Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"v".to_vec())],
                }),
                _ => Err(format!("unexpected syscall 0x{api:08x}")),
            }
        },
    )
    .expect("storage context token script should execute through the direct host path");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"v".to_vec())]);
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], (get_context, Vec::new()));
    assert_eq!(
        calls[1],
        (
            put,
            vec![
                StackValue::ByteString(token.clone()),
                StackValue::ByteString(b"k".to_vec()),
                StackValue::ByteString(b"v".to_vec()),
            ],
        )
    );
    assert_eq!(
        calls[2],
        (
            get,
            vec![
                StackValue::ByteString(token),
                StackValue::ByteString(b"k".to_vec()),
            ],
        )
    );
}

#[test]
fn storage_context_token_round_trips_across_syscalls_in_ffi_path() {
    let script = build_storage_context_round_trip_script();
    let token = storage_context_token(0x1234_5678, false);
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let put = neo_riscv_abi::interop_hash("System.Storage.Put");
    let get = neo_riscv_abi::interop_hash("System.Storage.Get");
    let mut state = Box::new(FfiStorageContextState {
        calls: Vec::new(),
        token: token.clone(),
    });
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            0,
            0,
            ptr::null(),
            0,
            (&mut *state) as *mut FfiStorageContextState as *mut c_void,
            ffi_storage_context_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute should be invoked");
    assert_eq!(output.state, 0, "ffi storage context script should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi result stack should decode");
    assert_eq!(stack, vec![StackValue::ByteString(b"v".to_vec())]);
    assert_eq!(state.calls.len(), 3);
    assert_eq!(state.calls[0], (get_context, Vec::new()));
    assert_eq!(
        state.calls[1],
        (
            put,
            vec![
                StackValue::ByteString(token.clone()),
                StackValue::ByteString(b"k".to_vec()),
                StackValue::ByteString(b"v".to_vec()),
            ],
        )
    );
    assert_eq!(
        state.calls[2],
        (
            get,
            vec![
                StackValue::ByteString(token),
                StackValue::ByteString(b"k".to_vec()),
            ],
        )
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn popitem_removes_last_array_element() {
    // Script: PUSH3, PUSH2, PUSH1, PUSH3, PACK, POPITEM, RET
    // PACK pops 3 items: 1, 2, 3 (top to bottom) → Array([1, 2, 3])
    // POPITEM pops last element → Array([1, 2]), element = 3
    let script = vec![
        0x13, // PUSH3
        0x12, // PUSH2
        0x11, // PUSH1
        0x13, // PUSH3 (count)
        0xc0, // PACK
        0xd4, // POPITEM
        0x40, // RET
    ];

    let result = execute_script(&script).expect("POPITEM on array should succeed");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack.len(), 2);
    // The popped element is 3 (last in the array)
    assert_eq!(result.stack[1], StackValue::Integer(3));
    // The array now has [1, 2]
    match &result.stack[0] {
        StackValue::Array(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], StackValue::Integer(1));
            assert_eq!(items[1], StackValue::Integer(2));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn popitem_removes_last_struct_element() {
    // Script: PUSH2, PUSH1, PUSH2, PACKSTRUCT, POPITEM, RET
    // PACKSTRUCT pops 2 items: 1, 2 → Struct([1, 2])
    // POPITEM pops last element → Struct([1]), element = 2
    let script = vec![
        0x12, // PUSH2
        0x11, // PUSH1
        0x12, // PUSH2 (count)
        0xbf, // PACKSTRUCT
        0xd4, // POPITEM
        0x40, // RET
    ];

    let result = execute_script(&script).expect("POPITEM on struct should succeed");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack.len(), 2);
    assert_eq!(result.stack[1], StackValue::Integer(2));
    match &result.stack[0] {
        StackValue::Struct(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], StackValue::Integer(1));
        }
        other => panic!("expected Struct, got {other:?}"),
    }
}

#[test]
fn callt_invokes_host_callback() {
    // Script: PUSHDATA1 "test", CALLT(token=42), RET
    let script = vec![
        0x0c, 4, // PUSHDATA1, length=4
        b't', b'e', b's', b't', 0x37, // CALLT
        0x2a, 0x00, // token = 42 (little-endian u16)
        0x40, // RET
    ];

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 0,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == (neo_riscv_guest::CALLT_MARKER | 42) {
                let mut new_stack = stack.to_vec();
                new_stack.pop(); // pop "test"
                new_stack.push(StackValue::Integer(99));
                Ok(HostCallbackResult { stack: new_stack })
            } else {
                Err(format!("unknown callt token {api}"))
            }
        },
    )
    .expect("CALLT should invoke host callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(99)]);
}

#[test]
fn callt_array_result_round_trips_through_locals_and_pickitem_in_host_runtime() {
    let script = vec![
        0x57, 0x02, 0x00, // INITSLOT 2 locals, 0 args
        0x37, 0x00, 0x00, // CALLT 0
        0x70, // STLOC0
        0x68, // LDLOC0
        0x10, // PUSH0
        0xce, // PICKITEM
        0x71, // STLOC1
        0x69, // LDLOC1
        0x40, // RET
    ];

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(
                        b"Hello World!".to_vec(),
                    )])],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("host runtime should preserve CALLT array results through locals and PICKITEM");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"Hello World!".to_vec())]
    );
}

#[test]
fn callt_array_result_round_trips_with_live_args_in_host_runtime() {
    let script = vec![
        0x57, 0x02, 0x04, // INITSLOT 2 locals, 4 args
        0x37, 0x00, 0x00, // CALLT 0
        0x70, // STLOC0
        0x68, // LDLOC0
        0x10, // PUSH0
        0xce, // PICKITEM
        0x71, // STLOC1
        0x69, // LDLOC1
        0x40, // RET
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
            StackValue::Integer(0),
            StackValue::Null,
            StackValue::ByteString(
                b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec(),
            ),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(
                        b"Hello World!".to_vec(),
                    )])],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("host runtime should preserve CALLT array results with live args");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"Hello World!".to_vec())]
    );
}

#[test]
fn callt_array_result_round_trips_with_live_args_in_ffi_host_runtime() {
    let script = vec![
        0x57, 0x02, 0x04, // INITSLOT 2 locals, 4 args
        0x37, 0x00, 0x00, // CALLT 0
        0x70, // STLOC0
        0x68, // LDLOC0
        0x10, // PUSH0
        0xce, // PICKITEM
        0x71, // STLOC1
        0x69, // LDLOC1
        0x40, // RET
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
        StackValue::Integer(0),
        StackValue::Null,
        StackValue::ByteString(b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            ptr::null_mut(),
            ffi_callt_array_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi stack should decode");
    assert_eq!(
        stack,
        vec![StackValue::ByteString(b"Hello World!".to_vec())]
    );

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn callt_array_result_survives_prior_syscall_with_live_args_in_host_runtime() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let script = vec![
        0x57,
        0x02,
        0x04, // INITSLOT 2 locals, 4 args
        0x41, // SYSCALL
        platform_api.to_le_bytes()[0],
        platform_api.to_le_bytes()[1],
        platform_api.to_le_bytes()[2],
        platform_api.to_le_bytes()[3],
        0x45, // DROP
        0x37,
        0x00,
        0x00, // CALLT 0
        0x70, // STLOC0
        0x68, // LDLOC0
        0x10, // PUSH0
        0xce, // PICKITEM
        0x71, // STLOC1
        0x69, // LDLOC1
        0x40, // RET
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
            StackValue::Integer(0),
            StackValue::Null,
            StackValue::ByteString(
                b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec(),
            ),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"NEO".to_vec())],
                });
            }
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(
                        b"Hello World!".to_vec(),
                    )])],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("host runtime should preserve CALLT array results after a prior syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"Hello World!".to_vec())]
    );
}

#[test]
fn callt_array_result_survives_prior_hash_syscall_with_live_args_in_host_runtime() {
    let hash_api = neo_riscv_abi::interop_hash("System.Runtime.GetCallingScriptHash");
    let script = vec![
        0x57,
        0x02,
        0x04, // INITSLOT 2 locals, 4 args
        0x41, // SYSCALL
        hash_api.to_le_bytes()[0],
        hash_api.to_le_bytes()[1],
        hash_api.to_le_bytes()[2],
        hash_api.to_le_bytes()[3],
        0x45, // DROP
        0x37,
        0x00,
        0x00, // CALLT 0
        0x70, // STLOC0
        0x68, // LDLOC0
        0x10, // PUSH0
        0xce, // PICKITEM
        0x71, // STLOC1
        0x69, // LDLOC1
        0x40, // RET
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
            StackValue::Integer(0),
            StackValue::Null,
            StackValue::ByteString(
                b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec(),
            ),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == hash_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(vec![0x58; 20])],
                });
            }
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(
                        b"Hello World!".to_vec(),
                    )])],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("host runtime should preserve CALLT array results after a prior hash syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"Hello World!".to_vec())]
    );
}

#[test]
fn oracle_on_response_success_path_executes_in_host_runtime() {
    let hash_api = neo_riscv_abi::interop_hash("System.Runtime.GetCallingScriptHash");
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let put = neo_riscv_abi::interop_hash("System.Storage.Put");
    let script = vec![
        0x57, 0x02, 0x04, 0x41, 0x39, 0x53, 0x6e, 0x3c, 0x0c, 0x14, 0x58, 0x87, 0x17, 0x11, 0x7e,
        0x0a, 0xa8, 0x10, 0x72, 0xaf, 0xab, 0x71, 0xd2, 0xdd, 0x89, 0xfe, 0x7c, 0x4b, 0x92, 0xfe,
        0x98, 0x26, 0x16, 0x0c, 0x11, 0x4e, 0x6f, 0x20, 0x41, 0x75, 0x74, 0x68, 0x6f, 0x72, 0x69,
        0x7a, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x21, 0x3a, 0x7a, 0x10, 0x98, 0x26, 0x2e, 0x0c, 0x22,
        0x4f, 0x72, 0x61, 0x63, 0x6c, 0x65, 0x20, 0x72, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65,
        0x20, 0x66, 0x61, 0x69, 0x6c, 0x75, 0x72, 0x65, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x63,
        0x6f, 0x64, 0x65, 0x20, 0x7a, 0x37, 0x01, 0x00, 0x8b, 0xdb, 0x28, 0x3a, 0x7b, 0x37, 0x02,
        0x00, 0x70, 0x68, 0x10, 0xce, 0x71, 0x69, 0x0c, 0x08, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e,
        0x73, 0x65, 0x41, 0x9b, 0xf6, 0x67, 0xce, 0x41, 0xe6, 0x3f, 0x18, 0x84, 0x40,
    ];
    let mut stored = Vec::new();

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
            StackValue::Integer(0),
            StackValue::Null,
            StackValue::ByteString(
                b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec(),
            ),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == hash_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(vec![
                        0x58, 0x87, 0x17, 0x11, 0x7e, 0x0a, 0xa8, 0x10, 0x72, 0xaf, 0xab, 0x71,
                        0xd2, 0xdd, 0x89, 0xfe, 0x7c, 0x4b, 0x92, 0xfe,
                    ])],
                });
            }
            if api == (neo_riscv_guest::CALLT_MARKER | 2) {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Array(vec![StackValue::ByteString(
                        b"Hello World!".to_vec(),
                    )])],
                });
            }
            if api == get_context {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(storage_context_token(0, false))],
                });
            }
            if api == put {
                stored = stack.to_vec();
                return Ok(HostCallbackResult { stack: vec![] });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("oracle success callback path should execute in host runtime");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
    assert_eq!(
        stored,
        vec![
            StackValue::ByteString(b"Hello World!".to_vec()),
            StackValue::ByteString(b"Response".to_vec()),
            StackValue::ByteString(storage_context_token(0, false)),
        ]
    );
}

#[test]
fn oracle_on_response_success_path_executes_in_ffi_host_runtime() {
    let script = vec![
        0x57, 0x02, 0x04, 0x41, 0x39, 0x53, 0x6e, 0x3c, 0x0c, 0x14, 0x58, 0x87, 0x17, 0x11, 0x7e,
        0x0a, 0xa8, 0x10, 0x72, 0xaf, 0xab, 0x71, 0xd2, 0xdd, 0x89, 0xfe, 0x7c, 0x4b, 0x92, 0xfe,
        0x98, 0x26, 0x16, 0x0c, 0x11, 0x4e, 0x6f, 0x20, 0x41, 0x75, 0x74, 0x68, 0x6f, 0x72, 0x69,
        0x7a, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x21, 0x3a, 0x7a, 0x10, 0x98, 0x26, 0x2e, 0x0c, 0x22,
        0x4f, 0x72, 0x61, 0x63, 0x6c, 0x65, 0x20, 0x72, 0x65, 0x73, 0x70, 0x6f, 0x6e, 0x73, 0x65,
        0x20, 0x66, 0x61, 0x69, 0x6c, 0x75, 0x72, 0x65, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x63,
        0x6f, 0x64, 0x65, 0x20, 0x7a, 0x37, 0x01, 0x00, 0x8b, 0xdb, 0x28, 0x3a, 0x7b, 0x37, 0x02,
        0x00, 0x70, 0x68, 0x10, 0xce, 0x71, 0x69, 0x0c, 0x08, 0x52, 0x65, 0x73, 0x70, 0x6f, 0x6e,
        0x73, 0x65, 0x41, 0x9b, 0xf6, 0x67, 0xce, 0x41, 0xe6, 0x3f, 0x18, 0x84, 0x40,
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"[\"Hello World!\"]".to_vec()),
        StackValue::Integer(0),
        StackValue::Null,
        StackValue::ByteString(b"https://api.jsonbin.io/v3/qs/6520ad3c12a5d3765988542a".to_vec()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut state = Box::new(FfiOracleSuccessState { stored: None });
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut *state) as *mut FfiOracleSuccessState as *mut c_void,
            ffi_oracle_success_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi stack should decode");
    assert!(stack.is_empty());
    assert_eq!(
        state.stored,
        Some(vec![
            StackValue::ByteString(b"Hello World!".to_vec()),
            StackValue::ByteString(b"Response".to_vec()),
            StackValue::ByteString(storage_context_token(0, false)),
        ])
    );

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn attribute_test_path_executes_in_ffi_host_runtime() {
    let script = vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ];
    let mut state = Box::new(FfiAttributeState {
        observed_checkwitness: None,
    });
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            ptr::null(),
            0,
            (&mut *state) as *mut FfiAttributeState as *mut c_void,
            ffi_attribute_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi stack should decode");
    assert_eq!(stack, vec![StackValue::Boolean(true)]);
    assert_eq!(
        state.observed_checkwitness,
        Some(vec![StackValue::ByteString(vec![0; 20])])
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn attribute_owner_constructor_helper_updates_array_in_host_runtime() {
    let script = vec![
        0x57, 0x00, 0x02, // INITSLOT 0 locals, 2 args
        0x79, // LDARG1
        0x37, 0x00, 0x00, // CALLT 0
        0xdb, 0x30, // CONVERT Buffer
        0xdb, 0x28, // CONVERT ByteString
        0x4a, // DUP
        0xd8, // ISNULL
        0x24, 0x09, // JMPIFNOT 9
        0x4a, // DUP
        0xca, // SIZE
        0x00, 0x14, // PUSHINT8 20
        0x28, 0x03, // JMPEQ 3
        0x3a, // THROW
        0x4a, // DUP
        0x78, // LDARG0
        0x10, // PUSH0
        0x51, // ROT
        0xd0, // SETITEM
        0x45, // DROP
        0x49, // CLEAR
        0x78, // LDARG0
        0x40, // RET
    ];

    let result = execute_script_with_host_and_stack(
        &script,
        vec![
            StackValue::Array(vec![StackValue::Null]),
            StackValue::ByteString(b"AAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_vec()),
            StackValue::Array(vec![StackValue::Null]),
        ],
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                assert_eq!(
                    stack,
                    &vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(b"AAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_vec()),
                    ]
                );
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("attribute constructor helper should update the array in host runtime");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![StackValue::ByteString(vec![0; 20])])]
    );
}

#[test]
fn attribute_owner_constructor_helper_updates_picked_array_in_host_runtime() {
    let mut script = vec![
        0x0b, // PUSHNULL
        0x11, // PUSH1
        0xc0, // PACK
        0x0c, 0x1c, // PUSHDATA1 length 28
    ];
    script.extend_from_slice(b"AAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    script.extend_from_slice(&[
        0x11, // PUSH1
        0x4d, // PICK
        0x34, 0x03, // CALL +3 -> helper
        0x40, // RET
        0x57, 0x00, 0x02, // INITSLOT 0 locals, 2 args
        0x79, // LDARG1
        0x37, 0x00, 0x00, // CALLT 0
        0xdb, 0x30, // CONVERT Buffer
        0xdb, 0x28, // CONVERT ByteString
        0x4a, // DUP
        0xd8, // ISNULL
        0x24, 0x09, // JMPIFNOT 9
        0x4a, // DUP
        0xca, // SIZE
        0x00, 0x14, // PUSHINT8 20
        0x28, 0x03, // JMPEQ 3
        0x3a, // THROW
        0x4a, // DUP
        0x78, // LDARG0
        0x10, // PUSH0
        0x51, // ROT
        0xd0, // SETITEM
        0x45, // DROP
        0x40, // RET
    ]);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                assert_eq!(
                    stack,
                    &vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(b"AAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_vec()),
                    ]
                );
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("attribute constructor caller/helper path should update the picked array");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![StackValue::ByteString(vec![0; 20])])]
    );
}

#[test]
fn attribute_test_path_executes_in_host_runtime() {
    let script = vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ];
    let mut observed_checkwitness = None;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }
            if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
                observed_checkwitness = Some(stack.to_vec());
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("attribute test path should execute in host runtime");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
    assert_eq!(
        observed_checkwitness,
        Some(vec![StackValue::ByteString(vec![0; 20])])
    );
}

#[test]
fn attribute_test_path_survives_prior_noop_call_in_host_runtime() {
    let target = vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ];
    let helper = vec![0x40]; // RET
    let helper_start = 6 + target.len();
    let mut script = vec![0x35, 0x00, 0x00, 0x00, 0x00, 0x49];
    script.extend_from_slice(&target);
    script.extend_from_slice(&helper);
    let offset = helper_start as i32;
    script[1..5].copy_from_slice(&offset.to_le_bytes());

    let mut observed_checkwitness = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }
            if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
                observed_checkwitness = Some(stack.to_vec());
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }
            Err(format!(
                "unexpected callback api 0x{api:08x}; stack={stack:?}"
            ))
        },
    )
    .expect("attribute test path should survive a prior noop CALL");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
    assert_eq!(
        observed_checkwitness,
        Some(vec![StackValue::ByteString(vec![0; 20])])
    );
}

#[test]
fn attribute_test_path_survives_prior_initsslot_call_in_host_runtime() {
    let target = vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ];
    let helper = vec![0x56, 0x04, 0x40]; // INITSSLOT 4; RET
    let helper_start = 6 + target.len();
    let mut script = vec![0x35, 0x00, 0x00, 0x00, 0x00, 0x49];
    script.extend_from_slice(&target);
    script.extend_from_slice(&helper);
    let offset = helper_start as i32;
    script[1..5].copy_from_slice(&offset.to_le_bytes());

    let mut observed_checkwitness = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }
            if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
                observed_checkwitness = Some(stack.to_vec());
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }
            Err(format!(
                "unexpected callback api 0x{api:08x}; stack={stack:?}"
            ))
        },
    )
    .expect("attribute test path should survive a prior INITSSLOT CALL");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
    assert_eq!(
        observed_checkwitness,
        Some(vec![StackValue::ByteString(vec![0; 20])])
    );
}

#[test]
fn attribute_test_path_executes_via_minimal_initialize_wrapper_in_host_runtime() {
    let target = vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ];
    let init = vec![0x56, 0x04, 0x40];
    let wrapper_len = 12usize;
    let target_offset = wrapper_len as i32 - 6; // target CALL_L offset from wrapper CALL at ip=6
    let init_offset = (wrapper_len + target.len()) as i32; // init CALL_L offset from wrapper start

    let mut script = vec![0x35];
    script.extend_from_slice(&init_offset.to_le_bytes());
    script.push(0x49); // CLEAR
    script.push(0x35);
    script.extend_from_slice(&target_offset.to_le_bytes());
    script.push(0x40); // RET
    script.extend_from_slice(&target);
    script.extend_from_slice(&init);

    let mut observed_checkwitness = None;
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }
            if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
                observed_checkwitness = Some(stack.to_vec());
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                });
            }
            Err(format!(
                "unexpected callback api 0x{api:08x}; stack={stack:?}"
            ))
        },
    )
    .expect("minimal initialize wrapper should preserve attribute test semantics");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
    assert_eq!(
        observed_checkwitness,
        Some(vec![StackValue::ByteString(vec![0; 20])])
    );
}

#[test]
fn callt_array_result_survives_prior_call_with_live_stack_in_host_runtime() {
    let mut script = vec![
        0x34, 0x00, // CALL helper (patched below)
        0x49, // CLEAR
        0x0b, // PUSHNULL
        0x11, // PUSH1
        0xc0, // PACK
        0x0c, 0x1c, // PUSHDATA1 length 28
    ];
    script.extend_from_slice(b"AAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    script.extend_from_slice(&[
        0x11, // PUSH1
        0x4d, // PICK
        0x34, 0x00, // CALL constructor helper (patched below)
        0x40, // RET
    ]);
    let helper_start = script.len();
    script.push(0x40); // helper: RET
    let constructor_helper_start = script.len();
    script.extend_from_slice(&[
        0x57, 0x00, 0x02, // constructor helper: INITSLOT 0 2
        0x79, // LDARG1
        0x37, 0x00, 0x00, // CALLT 0
        0xdb, 0x30, // CONVERT Buffer
        0xdb, 0x28, // CONVERT ByteString
        0x4a, // DUP
        0xd8, // ISNULL
        0x24, 0x09, // JMPIFNOT 9
        0x4a, // DUP
        0xca, // SIZE
        0x00, 0x14, // PUSHINT8 20
        0x28, 0x03, // JMPEQ 3
        0x3a, // THROW
        0x4a, // DUP
        0x78, // LDARG0
        0x10, // PUSH0
        0x51, // ROT
        0xd0, // SETITEM
        0x45, // DROP
        0x40, // RET
    ]);
    script[1] = i8::try_from(helper_start).expect("helper offset fits in i8") as u8;
    let constructor_call_ip = 38usize;
    script[constructor_call_ip + 1] =
        i8::try_from(constructor_helper_start as isize - constructor_call_ip as isize)
            .expect("constructor helper offset fits in i8") as u8;

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::CALLT_MARKER {
                return Ok(HostCallbackResult {
                    stack: vec![
                        StackValue::Array(vec![StackValue::Null]),
                        StackValue::ByteString(vec![0; 20]),
                    ],
                });
            }

            Err(format!(
                "unexpected callback api 0x{api:08x}; stack={stack:?}"
            ))
        },
    )
    .expect("CALLT constructor helper should survive a prior CALL");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![StackValue::ByteString(vec![0; 20])])]
    );
}

#[test]
fn test_try_catch_syscall_exception() {
    // Script: TRY 0x0a00, SYSCALL 0xdeaddead, ENDTRY 0x05, PUSH1, ENDTRY 0x02, PUSH2
    // TRY=0x3b, SYSCALL=0x41, ENDTRY=0x3d, PUSH1=0x11, PUSH2=0x12
    let script: Vec<u8> = vec![
        0x3b, 0x0a, 0x00, // TRY catch_offset=10, finally_offset=0
        0x41, 0xde, 0xad, 0xde, 0xad, // SYSCALL 0xaddeadde (le bytes)
        0x3d, 0x05, // ENDTRY offset=5
        0x11, // PUSH1
        0x3d, 0x02, // ENDTRY offset=2
        0x12, // PUSH2
    ];
    let ctx = RuntimeContext {
        trigger: 0x40,
        network: 0,
        address_version: 0,
        timestamp: None,
        gas_left: 0,
        exec_fee_factor_pico: 0,
    };
    let (result, trace) = debug_execute_script_with_host_and_stack(
        &script,
        Vec::new(),
        ctx,
        |_api, _ip, _ctx, _stack| Err("error".to_string()),
    )
    .unwrap();
    eprintln!("state: {:?}", result.state);
    eprintln!("stack len: {}", result.stack.len());
    for (i, item) in result.stack.iter().enumerate() {
        eprintln!("  stack[{i}]: {:?}", item);
    }
    eprintln!("trace: {:?}", trace);
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack.len(), 3);
}

#[test]
fn test_try_catch_throw_simple() {
    // TRY catch=7 finally=0, PUSH0, THROW, ENDTRY 3, PUSH1, ENDTRY 2, PUSH2
    // TRY=0x3b, PUSH0=0x10, THROW=0x3a, ENDTRY=0x3d, PUSH1=0x11, PUSH2=0x12
    let script: Vec<u8> = vec![
        0x3b, 0x07, 0x00, // ip=0: TRY catch_offset=7, finally_offset=0
        0x10, // ip=3: PUSH0
        0x3a, // ip=4: THROW
        0x3d, 0x03, // ip=5: ENDTRY offset=3
        0x11, // ip=7: PUSH1 (catch block)
        0x3d, 0x02, // ip=8: ENDTRY offset=2
        0x12, // ip=10: PUSH2
    ];
    let ctx = RuntimeContext {
        trigger: 0x40,
        network: 0,
        address_version: 0,
        timestamp: None,
        gas_left: 0,
        exec_fee_factor_pico: 0,
    };
    let result = execute_script_with_host(&script, ctx, |_api, _ip, _ctx, _stack| {
        Ok(HostCallbackResult { stack: Vec::new() })
    })
    .unwrap();
    eprintln!("state: {:?}", result.state);
    eprintln!("stack len: {}", result.stack.len());
    for (i, item) in result.stack.iter().enumerate() {
        eprintln!("  stack[{i}]: {:?}", item);
    }
    // After THROW: catch pushes error string, PUSH1 pushes 1, ENDTRY jumps to PUSH2
    // Expected: [Integer(2), Integer(1), ByteString("THROW")]
    assert_eq!(result.stack.len(), 3);
}

#[test]
fn gas_exhaustion_through_polkavm() {
    // Run a script with very low gas — should FAULT or return error mentioning gas/charge
    let result = execute_script_with_context(
        &[0x11, 0x12, 0x9e, 0x40], // PUSH1, PUSH2, ADD, RET
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 1,
            exec_fee_factor_pico: 1_000_000,
        },
    );

    match result {
        Err(e) => assert!(
            e.contains("Insufficient GAS") || e.contains("gas") || e.contains("charge"),
            "error should mention gas exhaustion: {e}"
        ),
        Ok(r) => assert_eq!(
            r.state,
            VmState::Fault,
            "should FAULT when gas is exhausted"
        ),
    }
}

#[test]
fn pointer_type_through_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Test.Pointer");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == neo_riscv_abi::interop_hash("System.Test.Pointer") {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::Pointer(42)],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should handle Pointer type through callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Pointer(42)]);
}

#[test]
fn biginteger_through_host_callback() {
    let syscall = neo_riscv_abi::interop_hash("System.Test.BigInt");
    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    // A 16-byte big integer value (little-endian)
    let big_value = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a];

    let expected_big = big_value.clone();
    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 0,
            exec_fee_factor_pico: 0,
        },
        move |api, _ip, _context, _stack| {
            if api == neo_riscv_abi::interop_hash("System.Test.BigInt") {
                Ok(HostCallbackResult {
                    stack: vec![StackValue::BigInteger(expected_big.clone())],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    )
    .expect("host runtime should handle BigInteger type through callback");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::BigInteger(big_value)]);
}

#[test]
fn block_78538_contract_deploy_does_not_trap() {
    // Mainnet block 78538 tx 0xf5d8a7... — this script pushes a large manifest (855 bytes)
    // and NEF (311 bytes) then calls System.Contract.Call("deploy"). The large data caused
    // a PolkaVM Trap in earlier versions due to bump allocator overflow on 32-bit.
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");

    // Simplified: push two large byte arrays + 4 Contract.Call args, then SYSCALL
    let manifest = vec![0x42u8; 855]; // 855-byte manifest
    let nef = vec![0x4e; 311]; // 311-byte NEF

    let mut script = Vec::new();
    // PUSHDATA2 manifest
    script.push(0x0d);
    script.extend_from_slice(&(manifest.len() as u16).to_le_bytes());
    script.extend_from_slice(&manifest);
    // PUSHDATA2 nef
    script.push(0x0d);
    script.extend_from_slice(&(nef.len() as u16).to_le_bytes());
    script.extend_from_slice(&nef);
    // PUSH2 + PACK → Array([nef, manifest])
    script.push(0x12); // PUSH2
    script.push(0xc1); // PACK
                       // PUSH15 (callFlags)
    script.push(0x1f);
    // PUSHDATA1 "deploy"
    script.push(0x0c);
    script.push(6);
    script.extend_from_slice(b"deploy");
    // PUSHDATA1 contract hash (20 bytes)
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[
        0xfd, 0xa3, 0xfa, 0x43, 0x46, 0xea, 0x53, 0x2a, 0x25, 0x8f, 0xc4, 0x97, 0xdd, 0xad, 0xdb,
        0x64, 0x37, 0xc9, 0xfd, 0xff,
    ]);
    // SYSCALL System.Contract.Call
    script.push(0x41);
    script.extend_from_slice(&contract_call.to_le_bytes());
    // RET
    script.push(0x40);

    let result = execute_script_with_host(
        &script,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, _stack| {
            if api == contract_call {
                // Mock: return an ~843-byte encoded result (matching real ContractState size)
                let result_item = StackValue::Array(vec![
                    StackValue::Map(vec![
                        (
                            StackValue::ByteString(b"name".to_vec()),
                            StackValue::ByteString(b"HashPuppies".to_vec()),
                        ),
                        (
                            StackValue::ByteString(b"groups".to_vec()),
                            StackValue::Array(vec![]),
                        ),
                    ]),
                    StackValue::Integer(0),
                    StackValue::ByteString(vec![0u8; 200]),
                    StackValue::ByteString(vec![0u8; 300]),
                    StackValue::Array(vec![
                        StackValue::ByteString(vec![0u8; 50]),
                        StackValue::ByteString(vec![0u8; 50]),
                        StackValue::ByteString(vec![0u8; 50]),
                        StackValue::ByteString(vec![0u8; 50]),
                    ]),
                ]);
                Ok(HostCallbackResult {
                    stack: vec![result_item],
                })
            } else {
                Err(format!("unexpected syscall 0x{api:08x}"))
            }
        },
    );

    match result {
        Ok(r) => {
            assert_eq!(r.state, VmState::Halt, "script should halt normally");
            assert_eq!(r.stack.len(), 1, "should have 1 result item on stack");
        }
        Err(e) => {
            if e.contains("Trap") {
                eprintln!("TRAP ERROR: {e}");
                panic!("PolkaVM Trap on large contract deploy script: {e}");
            }
            eprintln!("Non-trap error (acceptable): {e}");
        }
    }
}

unsafe extern "C" fn ffi_deploy_callback(
    _user_data: *mut c_void,
    api: u32,
    _ip: usize,
    _trigger: u8,
    _network: u32,
    _address_version: u8,
    _timestamp: u64,
    _gas_left: i64,
    _input_stack_ptr: *const neo_riscv_host::NativeStackItem,
    _input_stack_len: usize,
    output: *mut NativeHostResult,
) -> bool {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    if api != contract_call {
        return false;
    }
    // Return a complex result mimicking ContractState (~800 bytes)
    let result_stack = vec![StackValue::Array(vec![
        StackValue::Map(vec![(
            StackValue::ByteString(b"name".to_vec()),
            StackValue::ByteString(b"HashPuppies".to_vec()),
        )]),
        StackValue::Integer(0),
        StackValue::ByteString(vec![0u8; 300]),
        StackValue::ByteString(vec![0u8; 200]),
        StackValue::Array(vec![
            StackValue::ByteString(vec![0u8; 50]),
            StackValue::ByteString(vec![0u8; 50]),
        ]),
    ])];
    let (stack_ptr, stack_len) = build_native_stack_items(&result_stack);
    unsafe {
        *output = NativeHostResult {
            stack_ptr,
            stack_len,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_deploy_free_callback(
    _user_data: *mut c_void,
    result: *mut NativeHostResult,
) {
    if result.is_null() {
        return;
    }
    let result = unsafe { &mut *result };
    if !result.stack_ptr.is_null() {
        unsafe { free_native_stack_items(result.stack_ptr, result.stack_len) };
        result.stack_ptr = ptr::null_mut();
        result.stack_len = 0;
    }
}

#[test]
fn block_78538_ffi_path_does_not_trap() {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    let manifest = vec![0x42u8; 855];
    let nef = vec![0x4e; 311];

    let mut script = Vec::new();
    script.push(0x0d);
    script.extend_from_slice(&(manifest.len() as u16).to_le_bytes());
    script.extend_from_slice(&manifest);
    script.push(0x0d);
    script.extend_from_slice(&(nef.len() as u16).to_le_bytes());
    script.extend_from_slice(&nef);
    script.push(0x12); // PUSH2
    script.push(0xc1); // PACK
    script.push(0x1f); // PUSH15
    script.push(0x0c);
    script.push(6);
    script.extend_from_slice(b"deploy");
    script.push(0x0c);
    script.push(20);
    script.extend_from_slice(&[0xfd; 20]);
    script.push(0x41);
    script.extend_from_slice(&contract_call.to_le_bytes());
    script.push(0x40);

    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
    };

    let success = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,           // initial_ip
            0x40,        // trigger
            860833102,   // network
            53,          // address_version
            0,           // timestamp
            100_000_000, // gas_left
            0,           // exec_fee_factor_pico
            ptr::null(), // initial_stack
            0,
            ptr::null_mut(), // user_data
            ffi_deploy_callback,
            ffi_deploy_free_callback,
            &mut output,
        )
    };

    assert!(success, "FFI execution should not crash");

    if !output.error_ptr.is_null() && output.error_len > 0 {
        let error = unsafe {
            String::from_utf8_lossy(slice::from_raw_parts(output.error_ptr, output.error_len))
                .to_string()
        };
        if error.contains("Trap") {
            unsafe { neo_riscv_free_execution_result(&mut output) };
            panic!("FFI path Trap: {error}");
        }
        eprintln!("FFI error (non-trap): {error}");
    }

    unsafe { neo_riscv_free_execution_result(&mut output) };
}

/// Test: Execute a C#-compiled RISC-V native contract (Contract_Assignment)
/// This binary was generated by: C# → nccs --target riscv → Rust → polkatool link → .polkavm
#[test]
fn test_csharp_compiled_native_contract() {
    let polkavm_path = "/tmp/riscv-test-output/contract_assignment.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found (run C# compiler with --target riscv first)");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 1_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "testAssignment",
        vec![],
        context,
        |_api, _ip, _ctx, _stack| Ok(HostCallbackResult { stack: vec![] }),
    );

    match &result {
        Ok(r) => {
            eprintln!(
                "C# native contract executed: state={:?}, stack={:?}",
                r.state, r.stack
            );
        }
        Err(e) => {
            eprintln!("C# native contract error: {e}");
        }
    }
    // The contract should at least load and attempt execution without panicking
    assert!(
        result.is_ok() || result.as_ref().err().is_some_and(|e| !e.contains("Trap")),
        "Contract should not trap: {:?}",
        result
    );
}

/// Test: Execute Contract_MissingCheckWitness.unsafeUpdate via Rust host directly.
/// This bypasses C# FFI marshaling to isolate whether the bug is in the Rust host or C# layer.
#[test]
fn test_native_contract_missing_check_witness_unsafe_update() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    // Storage for tracking Put calls
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    let storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let storage_clone = storage.clone();

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "unsafeUpdate",
        vec![
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={}",
                api,
                stack.len()
            );
            match api {
                0xce67f69b => {
                    // Storage.GetContext - return integer 0
                    eprintln!("[TEST]   -> GetContext");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Integer(0)],
                    })
                }
                0x84183fe6 => {
                    // Storage.Put(context, key, value)
                    if stack.len() >= 3 {
                        let key = match &stack[1] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        let value = match &stack[2] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        eprintln!(
                            "[TEST]   -> Put key={:?} value={:?}",
                            String::from_utf8_lossy(&key),
                            String::from_utf8_lossy(&value)
                        );
                        storage_clone.lock().unwrap().insert(key, value);
                    } else {
                        eprintln!("[TEST]   -> Put: stack too short (len={})", stack.len());
                    }
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => {
                    eprintln!("[TEST]   -> unknown syscall");
                    Ok(HostCallbackResult { stack: vec![] })
                }
            }
        },
    );

    match &result {
        Ok(r) => {
            eprintln!(
                "[TEST] result: state={:?}, stack_len={}, fault={:?}",
                r.state,
                r.stack.len(),
                r.fault_message
            );
            eprintln!("[TEST] storage entries: {}", storage.lock().unwrap().len());
            for (k, v) in storage.lock().unwrap().iter() {
                eprintln!(
                    "[TEST]   {:?} = {:?}",
                    String::from_utf8_lossy(k),
                    String::from_utf8_lossy(v)
                );
            }
        }
        Err(e) => {
            eprintln!("[TEST] error: {e}");
        }
    }

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(
        storage.lock().unwrap().len(),
        1,
        "Storage should have 1 entry"
    );
    assert_eq!(
        storage.lock().unwrap().get(b"mykey".as_slice()),
        Some(&b"myvalue".to_vec()),
        "Storage should contain mykey=myvalue"
    );
}

#[test]
fn test_native_contract_echo_args() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "echoArgs",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |_api, _ip, _ctx, _stack| Ok(HostCallbackResult { stack: vec![] }),
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[ECHO] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    // Stack should have [myvalue, mykey, myaccount] (loaded in reverse order: arg2, arg1, arg0)
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_echo_after_bridge() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "echoAfterBridge",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |api, _ip, _ctx, stack| {
            eprintln!(
                "[ECHO1] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[ECHO1] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_echo_after_bridge_with_local() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "echoAfterBridgeWithLocal",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |api, _ip, _ctx, stack| {
            eprintln!(
                "[ECHOWL] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[ECHOWL] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_echo_after_2_bridges() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "echoAfter2Bridges",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |api, _ip, _ctx, stack| {
            eprintln!(
                "[ECHO2] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                0xce67f69b => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(0)],
                }),
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[ECHO2] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_echo_after_2_bridges_local() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "echoAfter2BridgesLocal",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |api, _ip, _ctx, stack| {
            eprintln!(
                "[ECHOL] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                0xce67f69b => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(0)],
                }),
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[ECHOL] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_two_bridges_no_host() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "twoBridgesNoHost",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        |api, _ip, _ctx, _stack| {
            panic!(
                "twoBridgesNoHost should NOT call any host callbacks, but got api=0x{:08x}",
                api
            );
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TWOBRIDGES] state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, VmState::Halt, "Should HALT");
    assert_eq!(r.stack.len(), 3, "Stack should have 3 items");
    assert_eq!(
        r.stack[0],
        StackValue::ByteString(b"myvalue".to_vec()),
        "arg[2] should be myvalue"
    );
    assert_eq!(
        r.stack[1],
        StackValue::ByteString(b"mykey".to_vec()),
        "arg[1] should be mykey"
    );
    assert_eq!(
        r.stack[2],
        StackValue::ByteString(b"myaccount".to_vec()),
        "arg[0] should be myaccount"
    );
}

#[test]
fn test_native_contract_missing_check_witness_safe_update() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    let storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let storage_clone = storage.clone();

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "safeUpdate",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()), // account for CheckWitness
            StackValue::ByteString(b"mykey".to_vec()),     // storage key
            StackValue::ByteString(b"myvalue".to_vec()),   // storage value
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => {
                    // CheckWitness - return true
                    eprintln!("[TEST]   -> CheckWitness -> true");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Boolean(true)],
                    })
                }
                0xce67f69b => {
                    // Storage.GetContext - return integer 0
                    eprintln!("[TEST]   -> GetContext");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Integer(0)],
                    })
                }
                0x84183fe6 => {
                    // Storage.Put(context, key, value)
                    if stack.len() >= 3 {
                        let key = match &stack[1] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        let value = match &stack[2] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        eprintln!(
                            "[TEST]   -> Put key={:?} value={:?}",
                            String::from_utf8_lossy(&key),
                            String::from_utf8_lossy(&value)
                        );
                        storage_clone.lock().unwrap().insert(key, value);
                    } else {
                        eprintln!("[TEST]   -> Put SKIPPED: stack too short");
                    }
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();

    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(
        storage.lock().unwrap().len(),
        1,
        "Storage should have 1 entry"
    );
    assert_eq!(
        storage.lock().unwrap().get(b"mykey".as_slice()),
        Some(&b"myvalue".to_vec()),
        "Storage should contain mykey=myvalue"
    );
}

#[test]
fn test_native_contract_checkwitness_simple() {
    let polkavm_path = "/tmp/riscv-test-output/contract_testcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "testCheckWitness",
        vec![StackValue::ByteString(b"testaddr".to_vec())],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={}",
                api,
                stack.len()
            );
            match api {
                0x8cec27f8 => {
                    // CheckWitness - return true
                    eprintln!("[TEST]   -> CheckWitness -> true");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Boolean(true)],
                    })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TEST] result: state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert!(!r.stack.is_empty(), "Result stack should not be empty");
    assert_eq!(
        r.stack[0],
        StackValue::Boolean(true),
        "CheckWitness should return true"
    );
}

#[test]
fn test_native_contract_checkwitness_init3() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "testCheckWitness3",
        vec![
            StackValue::Boolean(true),
            StackValue::Boolean(true),
            StackValue::Boolean(true),
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={}",
                api,
                stack.len()
            );
            match api {
                0x8cec27f8 => {
                    eprintln!("[TEST]   -> CheckWitness -> true");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Boolean(true)],
                    })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TEST] result: state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
}

#[test]
fn test_native_contract_safe_update_minimal_args() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    // Use only Boolean(true) as all 3 args — minimize encoding differences
    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "safeUpdate",
        vec![
            StackValue::Boolean(true),
            StackValue::Boolean(true),
            StackValue::Boolean(true),
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => {
                    eprintln!("[TEST]   -> CheckWitness -> true");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Boolean(true)],
                    })
                }
                0xce67f69b => {
                    eprintln!("[TEST]   -> GetContext");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Integer(0)],
                    })
                }
                0x84183fe6 => {
                    eprintln!("[TEST]   -> Put");
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TEST] result: state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
}

#[test]
fn test_native_contract_unsafe_update_two_syscalls() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "unsafeUpdate",
        vec![
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={}",
                api,
                stack.len()
            );
            match api {
                0xce67f69b => {
                    eprintln!("[TEST]   -> GetContext");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Integer(0)],
                    })
                }
                0x84183fe6 => {
                    eprintln!("[TEST]   -> Put");
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TEST] result: state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
}

#[test]
fn test_native_contract_checkwitness_with_assert() {
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "testCheckWitness",
        vec![StackValue::ByteString(b"testaddr".to_vec())],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] callback: api=0x{:08x} stack_len={}",
                api,
                stack.len()
            );
            match api {
                0x8cec27f8 => {
                    eprintln!("[TEST]   -> CheckWitness -> true");
                    Ok(HostCallbackResult {
                        stack: vec![StackValue::Boolean(true)],
                    })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!("[TEST] result: state={:?} stack={:?}", r.state, r.stack);
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
}

#[test]
fn test_native_contract_checkwitness_then_put_hardcoded() {
    // Test: CheckWitness + Put with hardcoded values (no arg loading after syscalls)
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    let storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let storage_clone = storage.clone();

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "checkWitnessThenPut",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()),
            StackValue::ByteString(b"mykey".to_vec()),
            StackValue::ByteString(b"myvalue".to_vec()),
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] checkwitness_then_put: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                0xce67f69b => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(0)],
                }),
                0x84183fe6 => {
                    if stack.len() >= 3 {
                        let key = match &stack[1] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        let value = match &stack[2] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        eprintln!(
                            "[TEST]   -> Put key={:?} value={:?}",
                            String::from_utf8_lossy(&key),
                            String::from_utf8_lossy(&value)
                        );
                        storage_clone.lock().unwrap().insert(key, value);
                    }
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(
        storage.lock().unwrap().get(b"hardcoded_key".as_slice()),
        Some(&b"hardcoded_value".to_vec())
    );
}

#[test]
fn test_native_contract_checkwitness_getcontext_put() {
    // Test: CheckWitness + GetContext + Put with arg loading after syscalls
    let polkavm_path = "/tmp/riscv-test-output/contract_missingcheckwitness.polkavm";
    if !std::path::Path::new(polkavm_path).exists() {
        eprintln!("Skipping: {polkavm_path} not found");
        return;
    }
    let binary = std::fs::read(polkavm_path).expect("read polkavm binary");

    let context = RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 10_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    };

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    let storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let storage_clone = storage.clone();

    let result = neo_riscv_host::execute_native_contract(
        &binary,
        "checkWitnessGetContextPut",
        vec![
            StackValue::ByteString(b"myaccount".to_vec()), // account for CheckWitness
            StackValue::ByteString(b"mykey".to_vec()),     // storage key
            StackValue::ByteString(b"myvalue".to_vec()),   // storage value
        ],
        context,
        move |api, _ip, _ctx, stack| {
            eprintln!(
                "[TEST] checkwitness_getcontext_put: api=0x{:08x} stack_len={} stack={:?}",
                api,
                stack.len(),
                stack
            );
            match api {
                0x8cec27f8 => Ok(HostCallbackResult {
                    stack: vec![StackValue::Boolean(true)],
                }),
                0xce67f69b => Ok(HostCallbackResult {
                    stack: vec![StackValue::Integer(0)],
                }),
                0x84183fe6 => {
                    if stack.len() >= 3 {
                        let key = match &stack[1] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        let value = match &stack[2] {
                            StackValue::ByteString(b) => b.clone(),
                            _ => vec![],
                        };
                        eprintln!(
                            "[TEST]   -> Put key={:?} value={:?}",
                            String::from_utf8_lossy(&key),
                            String::from_utf8_lossy(&value)
                        );
                        storage_clone.lock().unwrap().insert(key, value);
                    }
                    Ok(HostCallbackResult { stack: vec![] })
                }
                _ => Ok(HostCallbackResult { stack: vec![] }),
            }
        },
    );

    assert!(result.is_ok(), "Contract should execute: {:?}", result);
    let r = result.unwrap();
    eprintln!(
        "[TEST] checkwitness_getcontext_put result: state={:?} stack={:?}",
        r.state, r.stack
    );
    assert_eq!(r.state, neo_riscv_abi::VmState::Halt, "Should HALT");
    assert_eq!(
        storage.lock().unwrap().len(),
        1,
        "Storage should have 1 entry"
    );
    assert_eq!(
        storage.lock().unwrap().get(b"mykey".as_slice()),
        Some(&b"myvalue".to_vec()),
        "Storage should contain mykey=myvalue"
    );
}
