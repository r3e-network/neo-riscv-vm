use neo_riscv_abi::{BackendKind, StackValue, VmState};
use neo_riscv_host::{
    debug_execute_script_with_host_and_stack, execute_script, execute_script_with_context,
    execute_script_with_host, execute_script_with_host_and_stack, execute_script_with_trigger,
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
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
    assert_eq!(observed, Some(expected));
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
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
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
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
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
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
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
            if api == 42 {
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
fn test_try_catch_syscall_exception() {
    // Script: TRY 0x0a00, SYSCALL 0xdeaddead, ENDTRY 0x05, PUSH1, ENDTRY 0x02, PUSH2
    // TRY=0x3b, SYSCALL=0x41, ENDTRY=0x3d, PUSH1=0x11, PUSH2=0x12
    let script: Vec<u8> = vec![
        0x3b, 0x0a, 0x00,           // TRY catch_offset=10, finally_offset=0
        0x41, 0xde, 0xad, 0xde, 0xad, // SYSCALL 0xaddeadde (le bytes)
        0x3d, 0x05,                  // ENDTRY offset=5
        0x11,                        // PUSH1
        0x3d, 0x02,                  // ENDTRY offset=2
        0x12,                        // PUSH2
    ];
    let ctx = RuntimeContext {
        trigger: 0x40,
        network: 0,
        address_version: 0,
        timestamp: None,
        gas_left: 0,
        exec_fee_factor_pico: 0,
    };
    let (result, trace) = debug_execute_script_with_host_and_stack(&script, Vec::new(), ctx, |_api, _ip, _ctx, _stack| {
        Err(format!("error"))
    }).unwrap();
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
        0x3b, 0x07, 0x00,  // ip=0: TRY catch_offset=7, finally_offset=0
        0x10,               // ip=3: PUSH0
        0x3a,               // ip=4: THROW
        0x3d, 0x03,         // ip=5: ENDTRY offset=3
        0x11,               // ip=7: PUSH1 (catch block)
        0x3d, 0x02,         // ip=8: ENDTRY offset=2
        0x12,               // ip=10: PUSH2
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
    }).unwrap();
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
    assert_eq!(
        result.stack,
        vec![StackValue::BigInteger(big_value)]
    );
}
