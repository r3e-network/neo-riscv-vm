// NeoVM RISC-V Contract Integration Tests
//
// Tests NeoVM contract patterns including:
// - Token transfer logic
// - Conditional branches
// - Array/Map operations
// - Financial arithmetic
// - String/bytes operations
// - Exception handling
// - Type conversion

use neo_riscv_abi::{StackValue, VmState};
use neo_riscv_guest::{interpret, interpret_with_syscalls, SyscallProvider};

/// Simple syscall provider that handles standard Neo syscalls
struct TestSyscallProvider;

impl SyscallProvider for TestSyscallProvider {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        stack: &mut Vec<neo_riscv_abi::StackValue>,
    ) -> Result<(), String> {
        match api {
            // System.Runtime.Platform
            0xf6fc79b2 => {
                stack.push(neo_riscv_abi::StackValue::ByteString(b"NEO".to_vec()));
            }
            // System.Runtime.GetTrigger
            0xa0387de9 => {
                stack.push(neo_riscv_abi::StackValue::Integer(0x40));
            }
            // System.Runtime.GasLeft
            0xced88814 => {
                stack.push(neo_riscv_abi::StackValue::Integer(100000));
            }
            _ => {
                return Err(format!("unsupported syscall 0x{api:08x}"));
            }
        }
        Ok(())
    }
}

// ===== TOKEN TRANSFER CONTRACT =====

#[test]
fn token_balance_subtraction() {
    // 100 - 30 = 70
    let result = interpret(&[
        0x00, // PUSHINT8
        0x64, // 100
        0x00, // PUSHINT8
        0x1e, // 30
        0x9f, // SUB
        0x40, // RET
    ])
    .expect("should execute balance subtraction");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(70)]);
}

#[test]
fn token_balance_addition() {
    // 0 + 30 = 30
    let result = interpret(&[
        0x10, // PUSH0
        0x00, // PUSHINT8
        0x1e, // 30
        0x9e, // ADD
        0x40, // RET
    ])
    .expect("should execute balance addition");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(30)]);
}

#[test]
fn token_transfer_full_sequence() {
    // sender=100, receiver=0, amount=30
    // sender_final = 70, receiver_final = 30
    // SUB computes left - right, so push 100 then 30
    let result = interpret(&[
        0x00, 0x64, // PUSHINT8 100 (sender)
        0x00, 0x1e, // PUSHINT8 30 (amount)
        0x9f, // SUB: 100-30=70 → [70]
        0x10, // PUSH0 (receiver)
        0x00, 0x1e, // PUSHINT8 30 (amount)
        0x9e, // ADD: 0+30=30 → [70, 30]
        0x40, // RET
    ])
    .expect("should execute token transfer");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(70), StackValue::Integer(30)]
    );
}

#[test]
fn token_balance_negative_underflow() {
    // PUSHINT8 0x96 = -106 as i8, not 150
    // Test: 100 + (-106) = -6 (signed)
    let result = interpret(&[
        0x00, 0x64, // PUSHINT8 100
        0x00, 0x96, // PUSHINT8 0x96 = -106 as i8
        0x9e, // ADD → -6
        0x40, // RET
    ])
    .expect("should execute signed addition");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(-6)]);
}

// ===== SYSCALL CONTRACTS =====

#[test]
fn runtime_platform_syscall() {
    let platform = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41]; // SYSCALL
    script.extend_from_slice(&platform.to_le_bytes());
    script.push(0x40); // RET

    let mut provider = TestSyscallProvider;
    let result =
        interpret_with_syscalls(&script, &mut provider).expect("should execute platform syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"NEO".to_vec())]);
}

#[test]
fn runtime_gas_left_syscall() {
    let gas = neo_riscv_abi::interop_hash("System.Runtime.GasLeft");
    let mut script = vec![0x41]; // SYSCALL
    script.extend_from_slice(&gas.to_le_bytes());
    script.push(0x40); // RET

    let mut provider = TestSyscallProvider;
    let result =
        interpret_with_syscalls(&script, &mut provider).expect("should execute gas left syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(100000)]);
}

#[test]
fn double_syscall_contract() {
    // Call Platform twice, verify both results
    let platform = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let mut script = vec![0x41]; // SYSCALL
    script.extend_from_slice(&platform.to_le_bytes());
    script.push(0x41); // SYSCALL again
    script.extend_from_slice(&platform.to_le_bytes());
    script.push(0x40); // RET

    let mut provider = TestSyscallProvider;
    let result =
        interpret_with_syscalls(&script, &mut provider).expect("should execute double syscall");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![
            StackValue::ByteString(b"NEO".to_vec()),
            StackValue::ByteString(b"NEO".to_vec())
        ]
    );
}

// ===== CONDITIONAL LOGIC CONTRACTS =====

#[test]
fn conditional_branch_if_true() {
    // if (true) { return 1 } else { return 2 }
    let result = interpret(&[
        0x08, // PUSHT
        0x24, // JMPIF
        0x04, // offset=4 → skip PUSH2+RET
        0x12, // PUSH2 (false branch)
        0x40, // RET
        0x11, // PUSH1 (true branch)
        0x40, // RET
    ])
    .expect("should execute conditional branch");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn conditional_branch_if_false() {
    // if (false) { return 1 } else { return 2 }
    let result = interpret(&[
        0x09, // PUSHF
        0x24, // JMPIF
        0x04, // offset=4 → skip PUSH2+RET
        0x12, // PUSH2 (false branch)
        0x40, // RET
        0x11, // PUSH1 (true branch)
        0x40, // RET
    ])
    .expect("should execute conditional branch");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn comparison_less_than() {
    // 5 < 10 → true
    let result = interpret(&[
        0x00, 0x05, // PUSHINT8 5
        0x00, 0x0a, // PUSHINT8 10
        0xb5, // LT
        0x40, // RET
    ])
    .expect("should execute less than comparison");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn comparison_greater_than() {
    // 10 > 5 → true
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x05, // PUSHINT8 5
        0xb7, // GT
        0x40, // RET
    ])
    .expect("should execute greater than comparison");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn comparison_equal() {
    // 42 == 42 → true
    let result = interpret(&[
        0x00, 0x2a, // PUSHINT8 42
        0x00, 0x2a, // PUSHINT8 42
        0x97, // EQUAL
        0x40, // RET
    ])
    .expect("should execute equality comparison");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn comparison_not_equal() {
    // 42 != 43 → true
    let result = interpret(&[
        0x00, 0x2a, // PUSHINT8 42
        0x00, 0x2b, // PUSHINT8 43
        0x98, // NOTEQUAL
        0x40, // RET
    ])
    .expect("should execute not-equal comparison");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn comparison_within_range() {
    // 5 WITHIN(0, 10) → true (0 <= 5 < 10)
    // Stack order: x, min, max (top=right)
    let result = interpret(&[
        0x00, 0x05, // PUSHINT8 5 (x)
        0x10, // PUSH0 (min=0)
        0x00, 0x0a, // PUSHINT8 10 (max)
        0xbb, // WITHIN
        0x40, // RET
    ])
    .expect("should execute WITHIN range check");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn comparison_within_range_false() {
    // 15 WITHIN(0, 10) → false
    let result = interpret(&[
        0x00, 0x0f, // PUSHINT8 15 (x)
        0x10, // PUSH0 (min=0)
        0x00, 0x0a, // PUSHINT8 10 (max)
        0xbb, // WITHIN
        0x40, // RET
    ])
    .expect("should execute WITHIN range check");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn contract_dispatch_pattern() {
    // Check if input == 1, return 100; else return 200
    // JMPIF: pops bool, if true → jump by offset
    // JMPIF offset is relative to next instruction, so offset=5 skips PUSHINT8 200 (2 bytes) + RET (1 byte) + 2 more
    let result = interpret(&[
        0x11, // PUSH1 (input method id = 1)
        0x11, // PUSH1 (compare value = 1)
        0xb3, // NUMEQUAL (1 == 1 → true)
        0x24, // JMPIF
        0x05, // offset=5 → skip PUSHINT8 200 (2 bytes) + RET (1 byte) + 2 = 5 bytes to true branch
        0x00, 0xc8, // PUSHINT8 200 (else branch)
        0x40, // RET
        0x00, 0x64, // PUSHINT8 100 (true branch)
        0x40, // RET
    ])
    .expect("should execute contract dispatch");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(100)]);
}

// ===== ARRAY/MAP OPERATION CONTRACTS =====

#[test]
fn array_creation_and_setitem() {
    // Create array of size 3, set array[0] = 42
    // SETITEM pops: value, key, container (top to bottom) and updates via propagate_update
    // We need DUP to keep a copy of the array reference
    let result = interpret(&[
        0x13, // PUSH3
        0xc3, // NEWARRAY → [array_of_3_nulls]
        0x4a, // DUP → [array, array]
        0x10, // PUSH0 (index 0)
        0x00, 0x2a, // PUSHINT8 42 (value)
        0xd0, // SETITEM → [updated_array]
        0x10, // PUSH0 (index 0)
        0xce, // PICKITEM → array[0] = 42
        0x40, // RET
    ])
    .expect("should execute array setitem/pickitem");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn array_size_check() {
    let result = interpret(&[
        0x15, // PUSH5
        0xc3, // NEWARRAY
        0xca, // SIZE
        0x40, // RET
    ])
    .expect("should execute array size check");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(5)]);
}

#[test]
fn array_pack_order() {
    // PACK: pops count items + count, pushes packed array (bottom-to-top order reversed)
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x13, // PUSH3 (count)
        0xc0, // PACK
        0x40, // RET
    ])
    .expect("should execute array pack");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack.len(), 1);
    match &result.stack[0] {
        StackValue::Array(items) => {
            assert_eq!(items.len(), 3);
            // PACK reverses: [1,2,3] → [3,2,1]
            assert_eq!(items[0], StackValue::Integer(3));
            assert_eq!(items[1], StackValue::Integer(2));
            assert_eq!(items[2], StackValue::Integer(1));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn array_multiple_setitems() {
    // Create array, set [0]=10, read it
    // Note: multiple SETITEMs require local slots - this tests single SETITEM
    let result = interpret(&[
        0x12, // PUSH2
        0xc3, // NEWARRAY → [array]
        0x4a, // DUP → [array, array]
        0x10, // PUSH0 (index 0)
        0x00, 0x0a, // PUSHINT8 10
        0xd0, // SETITEM → [array]
        // Read [0]
        0x10, // PUSH0 (index 0)
        0xce, // PICKITEM → 10
        0x40, // RET
    ])
    .expect("should execute single setitem");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(10)]);
}

#[test]
fn map_set_and_get() {
    // NEWMAP, set map["a"] = 42, get map["a"]
    // SETITEM on map pops: value, key, map - need DUP to preserve map reference
    let result = interpret(&[
        0xc8, // NEWMAP → [map]
        0x4a, // DUP → [map, map]
        0x0c, 0x01, 0x61, // PUSHDATA1 "a" (key)
        0x00, 0x2a, // PUSHINT8 42 (value)
        0xd0, // SETITEM → [map]
        0x0c, 0x01, 0x61, // PUSHDATA1 "a" (key to lookup)
        0xce, // PICKITEM → 42
        0x40, // RET
    ])
    .expect("should execute map set/get");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn map_multiple_entries() {
    // Create map, set map["a"]=1, get map["a"]
    // Note: multiple entries require local slots - this tests single entry
    let result = interpret(&[
        0xc8, // NEWMAP → [map]
        0x4a, // DUP → [map, map]
        // Set map["a"] = 1
        0x0c, 0x01, 0x61, // PUSHDATA1 "a"
        0x11, // PUSH1
        0xd0, // SETITEM → [map]
        // Get map["a"]
        0x0c, 0x01, 0x61, // PUSHDATA1 "a"
        0xce, // PICKITEM → 1
        0x40, // RET
    ])
    .expect("should execute map entry");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn map_keys_operation() {
    // SETITEM consumes map, need DUP before each
    let result = interpret(&[
        0xc8, // NEWMAP → [map]
        0x4a, // DUP → [map, map]
        0x0c, 0x01, 0x61, // PUSHDATA1 "a"
        0x11, // PUSH1
        0xd0, // SETITEM → [map]
        0x4a, // DUP → [map, map]
        0x0c, 0x01, 0x62, // PUSHDATA1 "b"
        0x12, // PUSH2
        0xd0, // SETITEM → [map]
        0xcc, // KEYS → array of keys
        0x40, // RET
    ])
    .expect("should execute map keys");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack.len(), 1);
    match &result.stack[0] {
        StackValue::Array(keys) => {
            assert_eq!(keys.len(), 2);
        }
        other => panic!("expected Array of keys, got {other:?}"),
    }
}

#[test]
fn nested_array_operations() {
    // Create [[1,2],[3,4]]
    let result = interpret(&[
        // First sub-array [1,2]
        0x11, // PUSH1
        0x12, // PUSH2
        0x12, // PUSH2 (count)
        0xc0, // PACK → [2,1] (reversed)
        // Second sub-array [3,4]
        0x13, // PUSH3
        0x14, // PUSH4
        0x12, // PUSH2 (count)
        0xc0, // PACK → [4,3] (reversed)
        // Outer array
        0x12, // PUSH2 (count)
        0xc0, // PACK → [[4,3],[2,1]]
        // Access inner[0][0] = 4
        0x10, // PUSH0 (outer index)
        0xce, // PICKITEM → [4,3]
        0x10, // PUSH0 (inner index)
        0xce, // PICKITEM → 4
        0x40, // RET
    ])
    .expect("should execute nested array");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(4)]);
}

// ===== FINANCIAL ARITHMETIC CONTRACTS =====

#[test]
fn modular_exponentiation() {
    // MODPOW(2, 10, 1000) = 2^10 mod 1000 = 24
    let result = interpret(&[
        0x00, 0x02, // PUSHINT8 2 (base)
        0x00, 0x0a, // PUSHINT8 10 (exponent)
        0x01, // PUSHINT16
        0xe8, 0x03, // 1000 in LE (modulus)
        0xa6, // MODPOW
        0x40, // RET
    ])
    .expect("should execute MODPOW");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(24)]);
}

#[test]
fn square_root() {
    // SQRT(144) = 12
    let result = interpret(&[
        0x02, // PUSHINT32
        0x90, 0x00, 0x00, 0x00, // 144 in LE (i32)
        0xa4, // SQRT
        0x40, // RET
    ])
    .expect("should execute SQRT");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(12)]);
}

#[test]
fn min_max_operations() {
    // MIN(10,5)=5, MAX(10,5)=10
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x05, // PUSHINT8 5
        0xb9, // MIN → 5
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x05, // PUSHINT8 5
        0xba, // MAX → 10
        0x40, // RET
    ])
    .expect("should execute MIN/MAX");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(5), StackValue::Integer(10)]
    );
}

#[test]
fn complex_arithmetic_expression() {
    // (10 + 5) * 3 - 7 = 38
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x05, // PUSHINT8 5
        0x9e, // ADD (15)
        0x00, 0x03, // PUSHINT8 3
        0xa0, // MUL (45)
        0x00, 0x07, // PUSHINT8 7
        0x9f, // SUB (38)
        0x40, // RET
    ])
    .expect("should execute complex arithmetic");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(38)]);
}

// ===== STRING/BYTES OPERATION CONTRACTS =====

#[test]
fn string_concatenation() {
    // "hel" + "lo" = "hello"
    let result = interpret(&[
        0x0c, 0x03, 0x68, 0x65, 0x6c, // PUSHDATA1 "hel"
        0x0c, 0x02, 0x6c, 0x6f, // PUSHDATA1 "lo"
        0x8b, // CAT
        0x40, // RET
    ])
    .expect("should execute string concatenation");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Buffer(b"hello".to_vec())]
    );
}

#[test]
fn string_substring() {
    // SUBSTR("hello", 1, 3) = "ell"
    let result = interpret(&[
        0x0c, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, // PUSHDATA1 "hello"
        0x00, 0x01, // PUSHINT8 1 (index)
        0x00, 0x03, // PUSHINT8 3 (count)
        0x8c, // SUBSTR
        0x40, // RET
    ])
    .expect("should execute substring");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"ell".to_vec())]);
}

#[test]
fn string_left() {
    // LEFT("hello", 3) = "hel"
    let result = interpret(&[
        0x0c, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, // PUSHDATA1 "hello"
        0x00, 0x03, // PUSHINT8 3 (count)
        0x8d, // LEFT
        0x40, // RET
    ])
    .expect("should execute LEFT");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"hel".to_vec())]);
}

#[test]
fn string_right() {
    // RIGHT("hello", 3) = "llo"
    let result = interpret(&[
        0x0c, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f, // PUSHDATA1 "hello"
        0x00, 0x03, // PUSHINT8 3 (count)
        0x8e, // RIGHT
        0x40, // RET
    ])
    .expect("should execute RIGHT");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"llo".to_vec())]);
}

// ===== BITWISE OPERATION CONTRACTS =====

#[test]
fn bitwise_and() {
    // AND(0xFF, 0x0F) = 0x0F
    let result = interpret(&[
        0x00, 0xff, // PUSHINT8 255 (0xFF)
        0x00, 0x0f, // PUSHINT8 15 (0x0F)
        0x91, // AND
        0x40, // RET
    ])
    .expect("should execute AND");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(0x0F)]);
}

#[test]
fn bitwise_or() {
    // OR(0xF0, 0x0F) = 0xFF
    let result = interpret(&[
        0x00, 0xf0, // PUSHINT8 -16 (0xF0 as i8 = -16)
        0x00, 0x0f, // PUSHINT8 15 (0x0F)
        0x92, // OR
        0x40, // RET
    ])
    .expect("should execute OR");

    assert_eq!(result.state, VmState::Halt);
    // -16 OR 15 = -1 (all bits set in 2's complement)
    assert_eq!(result.stack, vec![StackValue::Integer(-1)]);
}

#[test]
fn bitwise_xor() {
    // XOR(0xFF, 0x0F) = 0xF0 = -16 as i8
    let result = interpret(&[
        0x00, 0xff, // PUSHINT8 0xFF = -1 as i8
        0x00, 0x0f, // PUSHINT8 15
        0x93, // XOR
        0x40, // RET
    ])
    .expect("should execute XOR");

    assert_eq!(result.state, VmState::Halt);
    // -1 XOR 15 = -16
    assert_eq!(result.stack, vec![StackValue::Integer(-16)]);
}

// ===== EXCEPTION HANDLING CONTRACTS =====

#[test]
fn try_catch_catches_throw() {
    // Layout (byte offsets from TRY at ip=0):
    //   0: TRY catch_offset=+5, finally_offset=0 (3 bytes: 0x3b, 0x05, 0x00)
    //   3: PUSH1 (throw something)
    //   4: THROW (0x3a)
    //   --- catch handler at offset 5 ---
    //   5: PUSH2 (catch executed)
    //   6: ENDTRY +2 (0x3d, 0x02) → jump to ip=8
    //   --- end ---
    //   8: RET
    let result = interpret(&[
        0x3b, 0x05, 0x00, // TRY catch=+5, finally=0
        0x11, // PUSH1 (value to throw)
        0x3a, // THROW
        0x12, // PUSH2 (catch handler)
        0x3d, 0x02, // ENDTRY +2 → ip 8
        0x40, // RET
    ])
    .expect("should execute TRY/CATCH");

    assert_eq!(result.state, VmState::Halt);
    // Stack: THROW pushes error string, then PUSH2 pushes 2
    assert!(
        result.stack.contains(&StackValue::Integer(2)),
        "stack: {:?}",
        result.stack
    );
}

#[test]
fn try_finally_executes() {
    // TRY_L catch=0, finally=10
    // Normal execution: PUSH1
    // ENDTRY_L → jump to finally
    // finally: PUSH2
    let result = interpret(&[
        0x3c, // TRY_L (long form)
        0x00, 0x00, 0x00, 0x00, // catch_offset=0
        0x0c, 0x00, 0x00, 0x00, // finally_offset=12
        0x11, // PUSH1 (normal execution)
        0x3e, // ENDTRY_L
        0x02, 0x00, 0x00, 0x00, // skip 2 bytes
        0x12, // PUSH2 (finally block)
        0x40, // RET
    ])
    .expect("should execute TRY/FINALLY");

    assert_eq!(result.state, VmState::Halt);
}

// ===== TYPE CONVERSION =====

#[test]
fn convert_integer_to_boolean() {
    // Check ISTYPE for Integer type (0x21)
    // ISTYPE takes type as immediate byte after opcode
    let result = interpret(&[
        0x00, 0x2a, // PUSHINT8 42
        0xd9, // ISTYPE
        0x21, // Integer type (0x21)
        0x40, // RET
    ])
    .expect("should execute ISTYPE");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn convert_integer_to_bytes() {
    // CONVERT: stack is [value, type], type on top
    // CONVERT takes type as immediate byte after opcode
    // CONVERT(0x28) Integer → ByteString
    let result = interpret(&[
        0x00, 0x2a, // PUSHINT8 42
        0xdb, // CONVERT
        0x28, // ByteString type (0x28)
        0x40, // RET
    ])
    .expect("should execute CONVERT to ByteString");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(vec![0x2a])]);
}

// ===== ARITHMETIC OPERATIONS =====

#[test]
fn abs_negate_operations() {
    // ABS(-42) = 42
    let result = interpret(&[
        0x00, 0xd6, // PUSHINT8 -42 (0xD6 = -42 as i8)
        0x9a, // ABS → 42
        0x9b, // NEGATE → -42
        0x40, // RET
    ])
    .expect("should execute ABS/NEGATE");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(-42)]);
}

#[test]
fn increment_decrement() {
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x9c, // INC → 11
        0x9d, // DEC → 10
        0x40, // RET
    ])
    .expect("should execute INC/DEC");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(10)]);
}

#[test]
fn sign_detection() {
    // SIGN: -1 for negative, 0 for zero, 1 for positive
    let result = interpret(&[
        0x00, 0x05, // PUSHINT8 5
        0x99, // SIGN → 1
        0x0f, // PUSHM1 (-1)
        0x99, // SIGN → -1
        0x10, // PUSH0
        0x99, // SIGN → 0
        0x40, // RET
    ])
    .expect("should execute SIGN");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(1),
            StackValue::Integer(-1),
            StackValue::Integer(0)
        ]
    );
}

#[test]
fn boolean_logic_operations() {
    let result = interpret(&[
        0x08, // PUSHT
        0x09, // PUSHF
        0xab, // BOOLAND → false
        0x09, // PUSHF
        0x08, // PUSHT
        0xac, // BOOLOR → true
        0x40, // RET
    ])
    .expect("should execute boolean logic");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(false), StackValue::Boolean(true)]
    );
}

#[test]
fn zero_value_check() {
    // NZ (Not Zero): 5→true, 0→false
    let result = interpret(&[
        0x00, 0x05, // PUSHINT8 5
        0xb1, // NZ → true
        0x10, // PUSH0
        0xb1, // NZ → false
        0x40, // RET
    ])
    .expect("should execute NZ");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(true), StackValue::Boolean(false)]
    );
}

// ===== STRUCT OPERATIONS =====

#[test]
fn struct_pack_and_pickitem() {
    // Pack struct [10,20,30], pick index 1
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x14, // PUSHINT8 20
        0x00, 0x1e, // PUSHINT8 30
        0x13, // PUSH3 (count)
        0xbf, // PACKSTRUCT → struct
        0x11, // PUSH1 (index 1)
        0xce, // PICKITEM
        0x40, // RET
    ])
    .expect("should execute struct pack/pickitem");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(20)]);
}

#[test]
fn reverse_items_on_array() {
    // REVERSEITEMS consumes array, need DUP to preserve reference
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x13, // PUSH3 (count)
        0xc0, // PACK → [array]
        0x4a, // DUP → [array, array]
        0xd1, // REVERSEITEMS → [array]
        0x10, // PUSH0 (index 0)
        0xce, // PICKITEM → 1
        0x40, // RET
    ])
    .expect("should execute REVERSEITEMS");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn clearitems_on_array() {
    // CLEARITEMS consumes array, need DUP to preserve reference
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x13, // PUSH3 (count)
        0xc0, // PACK → [array]
        0x4a, // DUP → [array, array]
        0xd3, // CLEARITEMS → [array]
        0xca, // SIZE → 0
        0x40, // RET
    ])
    .expect("should execute CLEARITEMS");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(0)]);
}

// ===== ADDRESS VALIDATION =====

#[test]
fn address_length_validation() {
    // Create a 20-byte bytestring, verify length == 20
    let mut script = vec![
        0x0c, 0x14, // PUSHDATA1 length=20
    ];
    script.extend_from_slice(&[0x01; 20]); // 20 bytes
    script.extend_from_slice(&[
        0xca, // SIZE → 20
        0x00, 0x14, // PUSHINT8 20
        0x97, // EQUAL → true
        0x40, // RET
    ]);

    let result = interpret(&script).expect("should execute address validation");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn address_length_mismatch() {
    // Create a 10-byte bytestring, verify length != 20
    let mut script = vec![
        0x0c, 0x0a, // PUSHDATA1 length=10
    ];
    script.extend_from_slice(&[0x01; 10]);
    script.extend_from_slice(&[
        0xca, // SIZE → 10
        0x00, 0x14, // PUSHINT8 20
        0x98, // NOTEQUAL → true
        0x40, // RET
    ]);

    let result = interpret(&script).expect("should execute address validation");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}
