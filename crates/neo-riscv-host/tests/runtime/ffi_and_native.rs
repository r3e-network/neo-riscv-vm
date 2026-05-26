
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
        freed: 0,
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
    // POPITEM pops the collection and returns the removed item.
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
    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn popitem_removes_last_struct_element() {
    // Script: PUSH2, PUSH1, PUSH2, PACKSTRUCT, POPITEM, RET
    // PACKSTRUCT pops 2 items: 1, 2 → Struct([1, 2])
    // POPITEM pops the collection and returns the removed item.
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
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
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
fn callt_string_result_can_setitem_into_live_array_after_cat_in_host_runtime() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = Vec::new();
    for _ in 0..6 {
        script.push(0x41); // SYSCALL
        script.extend_from_slice(&platform_api.to_le_bytes());
        script.push(0x45); // DROP
    }
    script.push(0x40); // RET from initializer
    let method_ip = script.len();
    script.extend_from_slice(&[
        0x57, 0x00, 0x05, // INITSLOT 0 locals, 5 args
        0x78, // LDARG0 live array retained in argument slots
        0x0c, 0x0b, // PUSHDATA1 length 11
    ]);
    script.extend_from_slice(b"Blind Box #");
    script.extend_from_slice(&[
        0x7c, // LDARG4 numeric suffix
        0x37, 0x00, 0x00, // CALLT 0 -> itoa-like string conversion
        0x8b, // CAT
        0x4a, // DUP
        0x78, // LDARG0
        0x11, // PUSH1
        0x51, // ROT -> array, index, value for SETITEM
        0xd0, // SETITEM
        0x45, // DROP duplicated string
        0x40, // RET
    ]);

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        vec![
            StackValue::Integer(1),
            StackValue::Null,
            StackValue::Null,
            StackValue::Null,
            StackValue::Array(vec![
                StackValue::ByteString(vec![0; 20]),
                StackValue::Null,
                StackValue::Integer(0),
                StackValue::Integer(0),
                StackValue::Integer(1),
                StackValue::Null,
                StackValue::Null,
            ]),
        ],
        method_ip,
        0,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == platform_api {
                return Ok(HostCallbackResult {
                    stack: vec![StackValue::ByteString(b"NEO".to_vec())],
                });
            }
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                return Ok(HostCallbackResult { stack: Vec::new() });
            }
            if api == neo_riscv_guest::CALLT_MARKER {
                let mut next = stack.to_vec();
                let value = next.pop().expect("CALLT input should include an integer");
                assert_eq!(value, StackValue::Integer(1));
                next.push(StackValue::ByteString(b"1".to_vec()));
                return Ok(HostCallbackResult { stack: next });
            }

            Err(format!(
                "unexpected callback api 0x{api:08x}; stack={stack:?}"
            ))
        },
    )
    .expect("CALLT string result should survive CAT and SETITEM into a live array");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::ByteString(vec![0; 20]),
            StackValue::Buffer(b"Blind Box #1".to_vec()),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::Integer(1),
            StackValue::Null,
            StackValue::Null,
        ])]
    );
}

#[test]
fn contract_call_bytes_result_can_be_normalized_and_setitem_into_map() {
    let contract_call = neo_riscv_abi::interop_hash("System.Contract.Call");
    let mut script = vec![
        0x56, 0x06, // INITSSLOT 6
        0x0c, 0x08, // PUSHDATA1 "deployed"
    ];
    script.extend_from_slice(b"deployed");
    script.extend_from_slice(&[
        0x60, // STSFLD0
        0x0c, 0x0e, // PUSHDATA1 "AUTH_ADDRESSES"
    ]);
    script.extend_from_slice(b"AUTH_ADDRESSES");
    script.extend_from_slice(&[
        0x61, // STSFLD1
        0x0c, 0x21, // PUSHDATA1 33-byte key
    ]);
    script.extend_from_slice(&[0x03; 33]);
    script.extend_from_slice(&[
        0xdb, 0x28, // CONVERT ByteString
        0x62, // STSFLD2
        0x0c, 0x22, // PUSHDATA1 address
    ]);
    script.extend_from_slice(b"Nc6LJ79RodHzaz5BghHGChMZYRa9GqJvES");
    script.extend_from_slice(&[
        0x11, // PUSH1
        0xc0, // PACK
        0x0c, 0x01, 0x0f, // PUSHDATA1 CallFlags.All
        0x0c, 0x0c, // PUSHDATA1 "base58Decode"
    ]);
    script.extend_from_slice(b"base58Decode");
    script.extend_from_slice(&[
        0x0c, 0x14, // PUSHDATA1 StdLib hash
    ]);
    script.extend_from_slice(&[0x55; 20]);
    script.extend_from_slice(&[
        0x41, // SYSCALL System.Contract.Call
    ]);
    script.extend_from_slice(&contract_call.to_le_bytes());
    script.extend_from_slice(&[
        0x4a, // DUP
        0xca, // SIZE
        0x0c, 0x01, 0x14, // PUSHDATA1 0x14
        0xdb, 0x21, // CONVERT Integer
        0x2c, 0x08, // JMPGT to SUBSTR path
        0x4a, // DUP
        0xca, // SIZE
        0x9d, // DEC
        0x8e, // RIGHT
        0x22, 0x08, // JMP to post-normalize
        0x11, // PUSH1
        0x0c, 0x01, 0x14, // PUSHDATA1 0x14
        0xdb, 0x21, // CONVERT Integer
        0x8c, // SUBSTR
        0xdb, 0x28, // CONVERT ByteString
        0x4a, // DUP
        0xd9, 0x21, // ISTYPE Integer
        0x26, 0x2e, // JMPIFNOT to size assertion
        0x4a, // DUP
        0x10, // PUSH0
        0xb8, // GE
        0x39, // ASSERT
        0x4a, // DUP
        0xca, // SIZE
        0x00, 0x14, // PUSHINT8 20
        0x4b, // OVER
        0x4b, // OVER
        0x2e, 0x1e, // JMPGE
        0x0c, 0x14, // PUSHDATA1 20 zero bytes
    ]);
    script.extend_from_slice(&[0u8; 20]);
    script.extend_from_slice(&[
        0x53, // REVERSE3
        0x9f, // SUB
        0x8d, // LEFT
        0x8b, // CAT
        0x22, 0x04, // JMP
        0x45, // DROP
        0x45, // DROP
        0xdb, 0x28, // CONVERT ByteString
        0x4a, // DUP
        0xca, // SIZE
        0x00, 0x14, // PUSHINT8 20
        0xb3, // NUMEQUAL
        0x39, // ASSERT
        0x63, // STSFLD3
        0xc8, // NEWMAP
        0x4a, // DUP
        0x10, // PUSH0
        0x0c, 0x14, // PUSHDATA1 20-byte value
    ]);
    script.extend_from_slice(&[0x22; 20]);
    script.extend_from_slice(&[
        0xd0, // SETITEM
        0x4a, // DUP
        0x11, // PUSH1
        0x0c, 0x14, // PUSHDATA1 20-byte value
    ]);
    script.extend_from_slice(&[0x23; 20]);
    script.extend_from_slice(&[
        0xd0, // SETITEM
        0x4a, // DUP
        0x12, // PUSH2
        0x0c, 0x14, // PUSHDATA1 20-byte value
    ]);
    script.extend_from_slice(&[0x24; 20]);
    script.extend_from_slice(&[
        0xd0, // SETITEM
        0x4a, // DUP
        0x13, // PUSH3
        0x0c, 0x14, // PUSHDATA1 20-byte value
    ]);
    script.extend_from_slice(&[0x25; 20]);
    script.extend_from_slice(&[
        0xd0, // SETITEM
        0x40, // RET
    ]);

    let expected_call_stack = vec![
        StackValue::Array(vec![StackValue::ByteString(
            b"Nc6LJ79RodHzaz5BghHGChMZYRa9GqJvES".to_vec(),
        )]),
        StackValue::ByteString(vec![0x0f]),
        StackValue::ByteString(b"base58Decode".to_vec()),
        StackValue::ByteString(vec![0x55; 20]),
    ];
    let decoded_address = [
        0x35, 0xb1, 0x77, 0xcb, 0x21, 0x6f, 0x8d, 0x18, 0x72, 0xf9, 0x8e, 0xb3, 0x68, 0x24, 0xe9,
        0x75, 0x72, 0x6d, 0x1f, 0xf0, 0xc2, 0x79, 0x66, 0xba, 0x07,
    ];

    script.push(0x40); // RET from initializer
    let method_ip = script.len();
    script.push(0x40); // target method returns void

    let result = execute_script_with_host_and_stack_and_ip_and_initializer_with_result_limit(
        &script,
        vec![
            StackValue::ByteString(vec![0xaa; 2_157]),
            StackValue::ByteString(vec![0xbb; 3_155]),
        ],
        method_ip,
        0,
        0,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                return Ok(HostCallbackResult { stack: Vec::new() });
            }
            if api != contract_call {
                return Err(format!("unexpected callback api 0x{api:08x}"));
            }
            assert_eq!(stack, expected_call_stack.as_slice());
            Ok(HostCallbackResult {
                stack: vec![StackValue::ByteString(decoded_address.to_vec())],
            })
        },
    )
    .expect("Contract.Call bytes result should survive map SETITEM normalization flow");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
}

#[test]
fn ffi_callt_string_result_can_setitem_into_live_array_after_cat() {
    let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = Vec::new();
    for _ in 0..6 {
        script.push(0x41); // SYSCALL
        script.extend_from_slice(&platform_api.to_le_bytes());
        script.push(0x45); // DROP
    }
    script.push(0x40); // RET from initializer
    let method_ip = script.len();
    script.extend_from_slice(&[
        0x57, 0x00, 0x05, // INITSLOT 0 locals, 5 args
        0x78, // LDARG0 live array retained in argument slots
        0x0c, 0x0b, // PUSHDATA1 length 11
    ]);
    script.extend_from_slice(b"Blind Box #");
    script.extend_from_slice(&[
        0x7c, // LDARG4 numeric suffix
        0x37, 0x00, 0x00, // CALLT 0 -> itoa-like string conversion
        0x8b, // CAT
        0x4a, // DUP
        0x78, // LDARG0
        0x11, // PUSH1
        0x51, // ROT -> array, index, value for SETITEM
        0xd0, // SETITEM
        0x45, // DROP duplicated string
        0x40, // RET
    ]);

    let initial_stack = vec![
        StackValue::Integer(1),
        StackValue::Null,
        StackValue::Null,
        StackValue::Null,
        StackValue::Array(vec![
            StackValue::ByteString(vec![0; 20]),
            StackValue::Null,
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::Integer(1),
            StackValue::Null,
            StackValue::Null,
        ]),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            method_ip,
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
            ffi_callt_string_setitem_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi stack should decode");
    assert_eq!(
        stack,
        vec![StackValue::Array(vec![
            StackValue::ByteString(vec![0; 20]),
            StackValue::Buffer(b"Blind Box #1".to_vec()),
            StackValue::Integer(0),
            StackValue::Integer(0),
            StackValue::Integer(1),
            StackValue::Null,
            StackValue::Null,
        ])]
    );

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn ffi_callt_retains_consumed_mutations_before_later_setitem() {
    let script = vec![
        0x57, 0x00, 0x01, // INITSLOT 0 locals, 1 arg
        0x78, // LDARG0
        0x10, // PUSH0
        0x11, // PUSH1
        0xd0, // SETITEM arg0[0] = 1; consumes arg0 and records mutation
        0x11, // PUSH1
        0x37, 0x00, 0x00, // CALLT 0 across a non-empty consumed_mutations vector
        0x45, // DROP CALLT string result
        0x78, // LDARG0
        0x11, // PUSH1
        0x12, // PUSH2
        0xd0, // SETITEM arg0[1] = 2
        0x78, // LDARG0
        0x40, // RET
    ];
    let initial_stack = vec![StackValue::Array(vec![StackValue::Null, StackValue::Null])];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
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
            ffi_callt_string_setitem_callback,
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
        vec![StackValue::Array(vec![
            StackValue::Integer(1),
            StackValue::Integer(2),
        ])]
    );

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
        neo_riscv_free_execution_result(&mut output);
    }
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
        freed: 0,
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
        freed: 0,
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

fn attribute_test_path_target_script() -> Vec<u8> {
    vec![
        0x58, 0xd8, 0x26, 0x28, 0x0b, 0x11, 0xc0, 0x0c, 0x1c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x3d, 0x11, 0x4d, 0x34, 0x08, 0x60, 0x58, 0x34, 0x21,
        0x08, 0x40, 0x57, 0x00, 0x02, 0x79, 0x37, 0x00, 0x00, 0xdb, 0x30, 0xdb, 0x28, 0x4a, 0xd8,
        0x24, 0x09, 0x4a, 0xca, 0x00, 0x14, 0x28, 0x03, 0x3a, 0x4a, 0x78, 0x10, 0x51, 0xd0, 0x45,
        0x40, 0x57, 0x00, 0x01, 0x78, 0x10, 0xce, 0x41, 0xf8, 0x27, 0xec, 0x8c, 0x24, 0x0e, 0x0c,
        0x09, 0x65, 0x78, 0x63, 0x65, 0x70, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x40,
    ]
}

fn attribute_test_path_with_static_slot_initialization() -> Vec<u8> {
    let target = attribute_test_path_target_script();
    let init = vec![0x56, 0x04, 0x40]; // INITSSLOT 4; RET
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
    script
}

#[test]
fn initializer_entrypoint_preserves_static_fields_in_host_runtime() {
    let script = [
        0x58, // LDSFLD0
        0x40, // RET
        0x56, 0x01, // INITSSLOT 1
        0x17, // PUSH7
        0x60, // STSFLD0
        0x40, // RET
    ];
    let mut init_complete_count = 0usize;

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        Vec::new(),
        0,
        2,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                init_complete_count += 1;
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("host runtime must preserve _initialize static fields for target method");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(7)]);
    assert_eq!(init_complete_count, 1);
}

#[test]
fn initializer_entrypoint_preserves_hash_static_fields_in_host_runtime() {
    let mut script = vec![
        0x5d, // LDSFLD5
        0x40, // RET
        0x56, 0x09, // INITSSLOT 9
        0x0c, 0x14, // PUSHDATA1 20
    ];
    let hash0 = vec![
        0xbd, 0xdf, 0x40, 0x98, 0xf8, 0x78, 0xc9, 0xfb, 0x33, 0xc0, 0x3f, 0xcf, 0x36, 0x82, 0xa6,
        0x04, 0x3c, 0xd5, 0x35, 0x86,
    ];
    script.extend_from_slice(&hash0);
    script.extend_from_slice(&[
        0x60, // STSFLD0
        0x0c, 0x14, // PUSHDATA1 20
        0xb4, 0x91, 0x39, 0xa1, 0x47, 0xf6, 0x77, 0x84, 0xd3, 0x13, 0x67, 0x13, 0x6e, 0xb5, 0x69,
        0x40, 0x31, 0xa5, 0x75, 0xfb, 0x61, // STSFLD1
        0x0c, 0x14, // PUSHDATA1 20
        0x2a, 0x4c, 0x9a, 0x4d, 0x40, 0x22, 0x67, 0x8b, 0x03, 0xef, 0x1b, 0xbe, 0x08, 0x34, 0xf9,
        0x66, 0x46, 0x0d, 0xc4, 0x48, 0x62, // STSFLD2
        0x0c, 0x14, // PUSHDATA1 20
        0x20, 0xf0, 0xbe, 0xa4, 0x50, 0xad, 0xa7, 0xb9, 0x03, 0xb8, 0x97, 0x49, 0xd7, 0xc9, 0xbb,
        0xc1, 0x60, 0xb1, 0x48, 0xcd, 0x63, // STSFLD3
        0x03, 0x00, 0x00, 0x8a, 0x5d, 0x78, 0x45, 0x63, 0x01, // PUSHINT64
        0x65, // STSFLD5
        0x0c, 0x07, b'e', b'n', b't', b'e', b'r', b'e', b'd', 0x64, // STSFLD4
        0x0c, 0x05, b'a', b's', b's', b'e', b't', 0x66, // STSFLD6
        0x0c, 0x08, b'c', b'o', b'n', b't', b'r', b'a', b'c', b't', 0x67, 0x07, // STSFLD 7
        0x0c, 0x0b, b't', b'o', b't', b'a', b'l', b'S', b'u', b'p', b'p', b'l', b'y', 0x67,
        0x08, // STSFLD 8
        0x40, // RET
    ]);
    let mut init_complete_count = 0usize;

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        Vec::new(),
        0,
        2,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                init_complete_count += 1;
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("initializer must preserve 20-byte hash static fields");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(0x0163_4578_5d8a_0000)]
    );
    assert_eq!(init_complete_count, 1);
}

#[test]
fn initializer_entrypoint_allows_deploy_to_compare_hash_static_fields_in_host_runtime() {
    let mut script = vec![
        0x57, 0x00, 0x02, // target: INITSLOT 0 locals, 2 args
        0x5a, // LDSFLD2
        0x0c, 0x01, 0x00, // PUSHDATA1 [0]
        0x8b, // CAT
        0xdb, 0x21, // CONVERT Integer
        0x5b, // LDSFLD3
        0x0c, 0x01, 0x00, // PUSHDATA1 [0]
        0x8b, // CAT
        0xdb, 0x21, // CONVERT Integer
        0xb5, // LT
        0x40, // RET
        0x56, 0x09, // initializer: INITSSLOT 9
        0x0c, 0x14, // PUSHDATA1 20
        0xbd, 0xdf, 0x40, 0x98, 0xf8, 0x78, 0xc9, 0xfb, 0x33, 0xc0, 0x3f, 0xcf, 0x36, 0x82, 0xa6,
        0x04, 0x3c, 0xd5, 0x35, 0x86, 0x60, // STSFLD0
        0x0c, 0x14, // PUSHDATA1 20
        0xb4, 0x91, 0x39, 0xa1, 0x47, 0xf6, 0x77, 0x84, 0xd3, 0x13, 0x67, 0x13, 0x6e, 0xb5, 0x69,
        0x40, 0x31, 0xa5, 0x75, 0xfb, 0x61, // STSFLD1
        0x0c, 0x14, // PUSHDATA1 20
        0x2a, 0x4c, 0x9a, 0x4d, 0x40, 0x22, 0x67, 0x8b, 0x03, 0xef, 0x1b, 0xbe, 0x08, 0x34, 0xf9,
        0x66, 0x46, 0x0d, 0xc4, 0x48, 0x62, // STSFLD2
        0x0c, 0x14, // PUSHDATA1 20
        0x20, 0xf0, 0xbe, 0xa4, 0x50, 0xad, 0xa7, 0xb9, 0x03, 0xb8, 0x97, 0x49, 0xd7, 0xc9, 0xbb,
        0xc1, 0x60, 0xb1, 0x48, 0xcd, 0x63, // STSFLD3
    ];
    script.extend_from_slice(&[
        0x40, // RET
    ]);

    let initializer_ip = 19;
    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        vec![StackValue::Null, StackValue::Boolean(true)],
        0,
        initializer_ip,
        RuntimeContext {
            trigger: 0x40,
            network: 860833102,
            address_version: 53,
            timestamp: None,
            gas_left: 10_000_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("initializer must allow target method to compare UInt160-sized integer conversions");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn initializer_entrypoint_continues_to_later_target_method_in_host_runtime() {
    let script = [
        0x56, 0x01, // initializer: INITSSLOT 1
        0x17, // PUSH7
        0x60, // STSFLD0
        0x40, // RET
        0x58, // target: LDSFLD0
        0x40, // RET
    ];
    let mut init_complete_count = 0usize;

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        Vec::new(),
        5,
        0,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                init_complete_count += 1;
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("initializer must continue into a target method located after it");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(7)]);
    assert_eq!(init_complete_count, 1);
}

#[test]
fn initializer_entrypoint_preserves_target_method_arguments_in_host_runtime() {
    let script = [
        0x57, 0x00, 0x02, // target: INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0x79, // LDARG1
        0x40, // RET
        0x56, 0x01, // initializer: INITSSLOT 1
        0x11, // PUSH1
        0x60, // STSFLD0
        0x40, // RET
    ];
    let witness = vec![0x41; 20];
    let mut init_complete_count = 0usize;

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        &script,
        vec![
            StackValue::Integer(22),
            StackValue::ByteString(witness.clone()),
        ],
        0,
        6,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _context, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                init_complete_count += 1;
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback api 0x{api:08x}"))
        },
    )
    .expect("initializer must not corrupt target method arguments");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(witness), StackValue::Integer(22)]
    );
    assert_eq!(init_complete_count, 1);
}

unsafe extern "C" fn ffi_initializer_only_callback(
    user_data: *mut c_void,
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
    if api != neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
        return false;
    }

    unsafe {
        *(user_data as *mut usize) += 1;
        *output = NativeHostResult {
            stack_ptr: ptr::null_mut(),
            stack_len: 0,
            error_ptr: ptr::null_mut(),
            error_len: 0,
        };
    }
    true
}

unsafe extern "C" fn ffi_initializer_witness_callback(
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
    let state = unsafe { &mut *(user_data as *mut FfiInitializerWitnessState) };
    if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
        state.init_complete_count += 1;
        unsafe {
            *output = NativeHostResult {
                stack_ptr: ptr::null_mut(),
                stack_len: 0,
                error_ptr: ptr::null_mut(),
                error_len: 0,
            };
        }
        return true;
    }

    if api == neo_riscv_abi::interop_hash("System.Runtime.CheckWitness") {
        state.observed_checkwitness = Some(
            unsafe { copy_test_native_stack_items(input_stack_ptr.cast_mut(), input_stack_len) }
                .unwrap_or_default(),
        );
        let (stack_ptr, stack_len) = build_native_stack_items(&[StackValue::Boolean(true)]);
        unsafe {
            *output = NativeHostResult {
                stack_ptr,
                stack_len,
                error_ptr: ptr::null_mut(),
                error_len: 0,
            };
        }
        return true;
    }

    false
}

unsafe extern "C" fn ffi_initializer_storage_put_callback(
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
    let state = unsafe { &mut *(user_data as *mut FfiInitializerStoragePutState) };
    if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
        state.init_complete_count += 1;
        unsafe {
            *output = NativeHostResult {
                stack_ptr: ptr::null_mut(),
                stack_len: 0,
                error_ptr: ptr::null_mut(),
                error_len: 0,
            };
        }
        return true;
    }

    if api == neo_riscv_abi::interop_hash("System.Storage.Put") {
        let stack = unsafe {
            copy_test_native_stack_items(input_stack_ptr.cast_mut(), input_stack_len)
                .unwrap_or_default()
        };
        let key = stack
            .iter()
            .filter_map(|item| match item {
                StackValue::ByteString(bytes) | StackValue::Buffer(bytes) => Some(bytes.clone()),
                _ => None,
            })
            .find(|bytes| bytes.ends_with(b"AuditFee") || bytes.ends_with(b"EpochDuration"))
            .unwrap_or_default();
        state.observed_keys.push(key);
        unsafe {
            *output = NativeHostResult {
                stack_ptr: ptr::null_mut(),
                stack_len: 0,
                error_ptr: ptr::null_mut(),
                error_len: 0,
            };
        }
        return true;
    }

    false
}

unsafe extern "C" fn ffi_initializer_storage_get_then_call_callback(
    user_data: *mut c_void,
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
    let state = unsafe { &mut *(user_data as *mut FfiInitializerStorageGetThenCallState) };
    let result_stack = if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
        state.init_complete_count += 1;
        Vec::new()
    } else if api == neo_riscv_abi::interop_hash("System.Storage.GetContext") {
        vec![StackValue::Integer(1)]
    } else if api == neo_riscv_abi::interop_hash("System.Storage.Get") {
        state.get_calls += 1;
        vec![StackValue::ByteString(vec![0x41; 20])]
    } else {
        return false;
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

fn build_static_prefix_storage_put_script() -> (Vec<u8>, usize, usize) {
    fn push_data(script: &mut Vec<u8>, bytes: &[u8]) {
        assert!(bytes.len() <= u8::MAX as usize);
        script.push(0x0c); // PUSHDATA1
        script.push(bytes.len() as u8);
        script.extend_from_slice(bytes);
    }

    fn emit_storage_put_call(script: &mut Vec<u8>, key: &[u8], value: &[u8]) -> usize {
        push_data(script, value);
        push_data(script, key);
        script.push(0x10); // PUSH0 storage context token for the test host.
        let call_ip = script.len();
        script.push(0x35); // CALL_L helper
        script.extend_from_slice(&0i32.to_le_bytes());
        call_ip
    }

    let mut script = vec![0x56, 0x06]; // INITSSLOT 6
    push_data(&mut script, b"config");
    script.extend_from_slice(&[
        0xdb, 0x30, // CONVERT Buffer
        0x65, // STSFLD5
        0x40, // RET
    ]);

    let target_ip = script.len();
    let first_call_ip = emit_storage_put_call(&mut script, b"AuditFee", &[0x80, 0xf0, 0xfa, 0x02]);
    let second_call_ip = emit_storage_put_call(&mut script, b"EpochDuration", &[0xf0, 0x00]);
    script.push(0x40); // RET

    let helper_ip = script.len();
    script.extend_from_slice(&[
        0x57, 0x02, 0x03, // INITSLOT 2 locals, 3 args
        0x79, // LDARG1
        0xdb, 0x30, // CONVERT Buffer
        0x70, // STLOC0
        0x5d, // LDSFLD5
        0x4a, // DUP
        0xd8, // ISNULL
        0x26, 0x05, // JMPIFNOT +5
        0x45, // DROP
        0x0c, 0x00, // PUSHDATA1 empty
        0x68, // LDLOC0
        0x8b, // CAT
        0x71, // STLOC1
        0x78, // LDARG0
        0x69, // LDLOC1
        0x7a, // LDARG2
        0x53, // REVERSE3
        0x41, // SYSCALL Storage.Put
    ]);
    script.extend_from_slice(&neo_riscv_abi::interop_hash("System.Storage.Put").to_le_bytes());
    script.extend_from_slice(&[
        0x21, // NOP, matching neo-go emitted syscall padding.
        0x40, // RET
    ]);

    for call_ip in [first_call_ip, second_call_ip] {
        let offset = helper_ip as i32 - call_ip as i32;
        script[call_ip + 1..call_ip + 5].copy_from_slice(&offset.to_le_bytes());
    }

    (script, target_ip, 0)
}

fn build_storage_get_then_call_l_script() -> (Vec<u8>, usize, usize) {
    let get_context = neo_riscv_abi::interop_hash("System.Storage.GetContext");
    let get = neo_riscv_abi::interop_hash("System.Storage.Get");

    let target_ip = 0;
    let mut script = vec![0x57, 0x01, 0x02]; // target: INITSLOT 1 local, 2 args
    script.push(0x41); // SYSCALL Storage.GetContext
    script.extend_from_slice(&get_context.to_le_bytes());
    script.extend_from_slice(&[
        0x11, // PUSH1
        0x88, // NEWBUFFER
        0x4a, // DUP, keep an alias to the buffer after SETITEM consumes one copy
        0x10, // PUSH0
        0x11, // PUSH1
        0xd0, // SETITEM
        0x45, // DROP retained buffer alias
        0x79, // LDARG1 storage key
        0x41, // SYSCALL Storage.Get
    ]);
    script.extend_from_slice(&get.to_le_bytes());
    script.extend_from_slice(&[
        0x70, // STLOC0
        0x68, // LDLOC0
        0x0b, // PUSHNULL
        0x97, // EQUAL
        0xaa, // NOT
        0x39, // ASSERT
        0x78, // LDARG0
    ]);
    let call_ip = script.len();
    script.push(0x35); // CALL_L helper
    script.extend_from_slice(&0i32.to_le_bytes());
    script.push(0x40); // RET

    let helper_ip = script.len();
    script.extend_from_slice(&[
        0x57, 0x00, 0x01, // INITSLOT 0 locals, 1 arg
        0x78, // LDARG0
        0x40, // RET
    ]);

    let initializer_ip = script.len();
    script.extend_from_slice(&[
        0x56, 0x01, // INITSSLOT 1
        0x11, // PUSH1
        0x60, // STSFLD0
        0x40, // RET
    ]);

    let offset = helper_ip as i32 - call_ip as i32;
    script[call_ip + 1..call_ip + 5].copy_from_slice(&offset.to_le_bytes());

    (script, target_ip, initializer_ip)
}

#[test]
fn initializer_static_field_prefix_survives_repeated_storage_puts_in_ffi_host_runtime() {
    let (script, target_ip, initializer_ip) = build_static_prefix_storage_put_script();
    let mut state = FfiInitializerStoragePutState {
        init_complete_count: 0,
        observed_keys: Vec::new(),
    };
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };
    let initial_stack = vec![StackValue::ByteString(vec![0x80, 0xf0, 0xfa, 0x02])];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            target_ip,
            initializer_ip,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut state) as *mut FfiInitializerStoragePutState as *mut c_void,
            ffi_initializer_storage_put_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };
    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    assert_eq!(state.init_complete_count, 1);
    assert_eq!(
        state.observed_keys,
        vec![b"configAuditFee".to_vec(), b"configEpochDuration".to_vec()]
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn initializer_storage_get_then_call_l_survives_alias_mutation_in_ffi_host_runtime() {
    let (script, target_ip, initializer_ip) = build_storage_get_then_call_l_script();
    let recipient = vec![0x33; 20];
    let initial_stack = vec![
        StackValue::ByteString(b"vote".to_vec()),
        StackValue::ByteString(recipient.clone()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut state = FfiInitializerStorageGetThenCallState {
        init_complete_count: 0,
        get_calls: 0,
    };
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            target_ip,
            initializer_ip,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut state) as *mut FfiInitializerStorageGetThenCallState as *mut c_void,
            ffi_initializer_storage_get_then_call_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };
    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi result stack should decode");
    assert_eq!(stack, vec![StackValue::ByteString(recipient)]);
    assert_eq!(state.init_complete_count, 1);
    assert_eq!(state.get_calls, 1);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn initializer_entrypoint_preserves_target_method_arguments_in_ffi_host_runtime() {
    let script = [
        0x57, 0x00, 0x02, // target: INITSLOT 0 locals, 2 args
        0x78, // LDARG0
        0x79, // LDARG1
        0x40, // RET
        0x56, 0x01, // initializer: INITSSLOT 1
        0x11, // PUSH1
        0x60, // STSFLD0
        0x40, // RET
    ];
    let witness = vec![0x41; 20];
    let initial_stack = vec![
        StackValue::Integer(22),
        StackValue::ByteString(witness.clone()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut init_complete_count = 0usize;
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            0,
            6,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut init_complete_count) as *mut usize as *mut c_void,
            ffi_initializer_only_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi result stack should decode");
    assert_eq!(
        stack,
        vec![StackValue::ByteString(witness), StackValue::Integer(22)]
    );
    assert_eq!(init_complete_count, 1);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn initializer_entrypoint_preserves_ldarg0_for_checkwitness_in_ffi_host_runtime() {
    let check_witness = neo_riscv_abi::interop_hash("System.Runtime.CheckWitness");
    let mut script = vec![
        0x57, 0x08, 0x0a, // target: INITSLOT 8 locals, 10 args
        0x0c, 0x09, // PUSHDATA1 "Forbidden"
    ];
    script.extend_from_slice(b"Forbidden");
    script.extend_from_slice(&[
        0x78, // LDARG0
        0x41, // SYSCALL Runtime.CheckWitness
    ]);
    script.extend_from_slice(&check_witness.to_le_bytes());
    script.extend_from_slice(&[
        0x40, // RET
        0x56, 0x01, // initializer: INITSSLOT 1
        0x11, // PUSH1
        0x60, // STSFLD0
        0x40, // RET
    ]);
    let initializer_ip = script.len() - 5;
    let sender = vec![
        0x46, 0xf6, 0xda, 0x5a, 0x7a, 0x14, 0x99, 0x50, 0x50, 0xae, 0xee, 0xe8, 0x8f, 0x32, 0x0a,
        0x2c, 0xb3, 0x93, 0xcf, 0x40,
    ];
    let initial_stack = vec![
        StackValue::ByteString(b"nspcc_logo".to_vec()),
        StackValue::ByteString(vec![0x4e; 662]),
        StackValue::ByteString(b"https://twitter.com/neospcc".to_vec()),
        StackValue::ByteString(b"None".to_vec()),
        StackValue::ByteString(b"https://github.com/nspcc-dev".to_vec()),
        StackValue::ByteString(b"org@nspcc.ru".to_vec()),
        StackValue::ByteString(b"https://nspcc.ru/en".to_vec()),
        StackValue::ByteString(b"Europe".to_vec()),
        StackValue::ByteString(b"NeoSPCC".to_vec()),
        StackValue::ByteString(sender.clone()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut state = FfiInitializerWitnessState {
        init_complete_count: 0,
        observed_checkwitness: None,
    };
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            0,
            initializer_ip,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut state) as *mut FfiInitializerWitnessState as *mut c_void,
            ffi_initializer_witness_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    assert_eq!(state.init_complete_count, 1);
    assert_eq!(
        state.observed_checkwitness,
        Some(vec![StackValue::ByteString(sender)])
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn initializer_entrypoint_preserves_large_ten_argument_stack_in_ffi_host_runtime() {
    let script = [
        0x57, 0x00, 0x0a, // target: INITSLOT 0 locals, 10 args
        0x78, // LDARG0
        0x7f, 0x09, // LDARG9
        0x40, // RET
        0x56, 0x01, // initializer: INITSSLOT 1
        0x11, // PUSH1
        0x60, // STSFLD0
        0x40, // RET
    ];
    let sender = vec![
        0x46, 0xf6, 0xda, 0x5a, 0x7a, 0x14, 0x99, 0x50, 0x50, 0xae, 0xee, 0xe8, 0x8f, 0x32, 0x0a,
        0x2c, 0xb3, 0x93, 0xcf, 0x40,
    ];
    let logo = b"nspcc_logo".to_vec();
    let initial_stack = vec![
        StackValue::ByteString(logo.clone()),
        StackValue::ByteString(vec![0x4e; 662]),
        StackValue::ByteString(b"https://twitter.com/neospcc".to_vec()),
        StackValue::ByteString(b"None".to_vec()),
        StackValue::ByteString(b"https://github.com/nspcc-dev".to_vec()),
        StackValue::ByteString(b"org@nspcc.ru".to_vec()),
        StackValue::ByteString(b"https://nspcc.ru/en".to_vec()),
        StackValue::ByteString(b"Europe".to_vec()),
        StackValue::ByteString(b"NeoSPCC".to_vec()),
        StackValue::ByteString(sender.clone()),
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut init_complete_count = 0usize;
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer(
            script.as_ptr(),
            script.len(),
            0,
            7,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut init_complete_count) as *mut usize as *mut c_void,
            ffi_initializer_only_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi result stack should decode");
    assert_eq!(
        stack,
        vec![StackValue::ByteString(sender), StackValue::ByteString(logo)]
    );
    assert_eq!(init_complete_count, 1);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn initializer_entrypoint_preserves_nested_array_arguments_in_ffi_host_runtime() {
    let mut script = vec![
        0x57, 0x02, 0x05, // target: INITSLOT 2 locals, 5 args
        0x79, // LDARG1
        0xd9, 0x40, // ISTYPE Array
        0x39, // ASSERT
        0x79, // LDARG1
        0x40, // RET
    ];
    let initializer_ip = script.len();
    script.extend_from_slice(&[
        0x56, 0x12, // initializer: INITSSLOT 18
        0x0c, 0x03, b'A', b'C', b'C', 0x60, // static 0
        0x0c, 0x05, b'G', b'H', b'O', b'S', b'T', 0x61, // static 1
        0x10, 0x62, // static 2
        0x0c, 0x08, b'd', b'e', b'p', b'l', b'o', b'y', b'e', b'd', 0x63, // static 3
        0x0c, 0x06, b'p', b'a', b'u', b's', b'e', b'd', 0x64, // static 4
        0x40, // RET
    ]);

    let royalty = br#"[{"address":"NbNNVPCHPrbxTQrv8aYAQ1WodyKF1AJKto","value":"10"}]"#.to_vec();
    let metadata = br#"{"name":"Mushroomy","description":"Simple mushroom at peace in the forest.","image":"ipfs://Qma6dpRndfkaQXHTDYK1isebGknSn57pYxueobRK6G2goc","tokenURI":"","attributes":[{"type":"Stinkybadass","value":"Stinkybadass","display":""}],"properties":{"has_locked":false,"creator":"NbNNVPCHPrbxTQrv8aYAQ1WodyKF1AJKto","royalties":1000,"type":1}}"#.to_vec();
    let royalties = StackValue::Array(vec![StackValue::ByteString(royalty); 10]);
    let locked = StackValue::Array(vec![StackValue::ByteString(Vec::new()); 10]);
    let metadata_items = StackValue::Array(vec![StackValue::ByteString(metadata); 10]);
    let owner = StackValue::ByteString(vec![
        0xa9, 0x88, 0x27, 0xa7, 0x6b, 0xfe, 0x6e, 0xe6, 0x45, 0x54, 0x6d, 0xde, 0x37, 0x8b, 0x83,
        0x7e, 0x0b, 0x6b, 0x4f, 0x78,
    ]);
    let initial_stack = vec![
        StackValue::ByteString(Vec::new()),
        royalties.clone(),
        locked,
        metadata_items.clone(),
        owner,
    ];
    let (initial_stack_ptr, initial_stack_len) = build_native_stack_items(&initial_stack);
    let mut init_complete_count = 0usize;
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let invoked = unsafe {
        neo_riscv_execute_script_with_host_and_initializer_and_result_limit(
            script.as_ptr(),
            script.len(),
            0,
            initializer_ip,
            1,
            0x40,
            0,
            53,
            0,
            100_000_000,
            0,
            initial_stack_ptr,
            initial_stack_len,
            (&mut init_complete_count) as *mut usize as *mut c_void,
            ffi_initializer_only_callback,
            ffi_mixed_free_callback,
            &mut output,
        )
    };

    unsafe {
        free_native_stack_items(initial_stack_ptr, initial_stack_len);
    }

    assert!(invoked, "ffi execute with initializer should be invoked");
    assert_eq!(output.state, 0, "ffi execution should halt");
    let stack = unsafe { copy_test_native_stack_items(output.stack_ptr, output.stack_len) }
        .expect("ffi result stack should decode");
    assert_eq!(stack, vec![metadata_items]);
    assert_eq!(init_complete_count, 1);

    unsafe {
        neo_riscv_free_execution_result(&mut output);
    }
}

#[test]
fn attribute_test_path_executes_in_ffi_host_runtime() {
    let script = attribute_test_path_with_static_slot_initialization();
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
        freed: 0,
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
    let script = attribute_test_path_with_static_slot_initialization();
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
    let target = attribute_test_path_with_static_slot_initialization();
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
    // After THROW: catch receives the original thrown StackItem, PUSH1 pushes 1,
    // and ENDTRY jumps to PUSH2.
    assert_eq!(result.stack.len(), 3);
}

#[test]
fn try_catch_catches_pickitem_type_error_in_host_runtime() {
    let script: &[u8] = &[
        0x57, 0x01, 0x00, // INITSLOT 1 local, 0 args
        0x3b, 0x08, 0x00, // TRY catch=+8, finally=0
        0x08, // PUSHT (non-indexable item)
        0x10, // PUSH0 (index)
        0xce, // PICKITEM -> catchable type error
        0x3d, 0x06, // ENDTRY +6 -> RET
        0x70, // STLOC0 (catch stores thrown message)
        0x11, // PUSH1
        0x3d, 0x02, // ENDTRY +2 -> RET
        0x40, // RET
    ];

    let result = execute_script_with_host(
        script,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, stack| Err(format!("unexpected callback 0x{api:08x}: {stack:?}")),
    )
    .expect("host runtime should catch PICKITEM type errors inside TRY");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn initializer_then_target_catches_pickitem_type_error_in_host_runtime() {
    let script: &[u8] = &[
        0x57, 0x01, 0x00, // target: INITSLOT 1 local, 0 args
        0x3b, 0x08, 0x00, // TRY catch=+8, finally=0
        0x08, // PUSHT (non-indexable item)
        0x10, // PUSH0 (index)
        0xce, // PICKITEM -> catchable type error
        0x3d, 0x06, // ENDTRY +6 -> RET
        0x70, // STLOC0 (catch stores thrown message)
        0x11, // PUSH1
        0x3d, 0x02, // ENDTRY +2 -> RET
        0x40, // RET
        0x56, 0x01, // initializer: INITSSLOT 1
        0x10, // PUSH0
        0x60, // STSFLD0
        0x40, // RET
    ];
    let mut init_complete_count = 0usize;

    let result = execute_script_with_host_and_stack_and_ip_and_initializer(
        script,
        Vec::new(),
        0,
        12,
        RuntimeContext {
            trigger: 0x40,
            network: 0,
            address_version: 53,
            timestamp: None,
            gas_left: 100_000_000,
            exec_fee_factor_pico: 0,
        },
        |api, _ip, _ctx, stack| {
            if api == neo_riscv_guest::INITIALIZER_COMPLETE_MARKER {
                init_complete_count += 1;
                return Ok(HostCallbackResult {
                    stack: stack.to_vec(),
                });
            }

            Err(format!("unexpected callback 0x{api:08x}: {stack:?}"))
        },
    )
    .expect("target method should catch PICKITEM type errors after _initialize co-execution");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
    assert_eq!(init_complete_count, 1);
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
fn ffi_script_gas_exhaustion_reports_consumed_fee() {
    let script = [
        0x11, 0x12, 0x9e, 0x40, // PUSH1, PUSH2, ADD, RET
    ];
    let mut output = NativeExecutionResult {
        fee_consumed_pico: 0,
        state: 0,
        stack_ptr: ptr::null_mut(),
        stack_len: 0,
        error_ptr: ptr::null_mut(),
        error_len: 0,
        freed: 0,
    };

    let success = unsafe {
        neo_riscv_execute_script_with_host(
            script.as_ptr(),
            script.len(),
            0,
            0x40,
            0,
            53,
            0,
            1,
            1_000_000,
            ptr::null(),
            0,
            ptr::null_mut(),
            ffi_error_callback,
            ffi_error_free_callback,
            &mut output,
        )
    };

    assert!(success, "FFI execution should return a result");
    assert_eq!(output.state, 1, "gas exhaustion should fault");
    assert!(
        output.fee_consumed_pico > 0,
        "gas fault should report consumed fee"
    );

    unsafe {
        neo_riscv_free_execution_result(&mut output);
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
fn pow_accepts_big_integer_result_through_host_runtime() {
    let result = execute_script(&[
        0x12, // PUSH2
        0x00, 0x40, // PUSHINT8 64
        0xa3, // POW -> 2^64
        0x40, // RET
    ])
    .expect("host runtime should support POW results wider than i64");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::BigInteger(vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ])]
    );
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
        freed: 0,
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

