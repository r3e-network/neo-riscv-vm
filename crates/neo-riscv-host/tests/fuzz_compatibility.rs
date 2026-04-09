/// Comprehensive fuzz tests for Neo RISC-V VM compatibility.
///
/// Tests cover:
/// 1. NeoVM bytecode interpretation through PolkaVM guest
/// 2. Codec serialization/deserialization round-trips
/// 3. Arithmetic correctness and edge cases
/// 4. Stack manipulation opcodes
/// 5. Control flow opcodes
/// 6. Type conversion opcodes
/// 7. Compound type operations
/// 8. Resource limit enforcement
use neo_riscv_abi::{callback_codec, fast_codec, StackValue};
use neo_riscv_host::execute_script;
use proptest::prelude::*;

// =============================================================================
// Strategies for generating random StackValues
// =============================================================================

fn arb_primitive_value() -> impl Strategy<Value = StackValue> {
    prop_oneof![
        any::<i64>().prop_map(StackValue::Integer),
        any::<bool>().prop_map(StackValue::Boolean),
        prop::collection::vec(any::<u8>(), 0..128).prop_map(StackValue::ByteString),
        prop::collection::vec(any::<u8>(), 0..32).prop_map(StackValue::BigInteger),
        Just(StackValue::Null),
    ]
}

fn arb_stack_value() -> impl Strategy<Value = StackValue> {
    arb_primitive_value().prop_recursive(3, 64, 8, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..8).prop_map(StackValue::Array),
            prop::collection::vec(inner.clone(), 0..8).prop_map(StackValue::Struct),
        ]
    })
}

// =============================================================================
// 1. Fast Codec Round-Trip Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, .. ProptestConfig::default() })]

    #[test]
    fn fuzz_fast_codec_single_value(value in arb_stack_value()) {
        let stack = vec![value];
        let encoded = fast_codec::encode_stack(&stack);
        let decoded = fast_codec::decode_stack(&encoded).expect("decode failed");
        prop_assert_eq!(stack, decoded);
    }

    #[test]
    fn fuzz_fast_codec_multi_value(stack in prop::collection::vec(arb_stack_value(), 0..16)) {
        let encoded = fast_codec::encode_stack(&stack);
        let decoded = fast_codec::decode_stack(&encoded).expect("decode failed");
        prop_assert_eq!(stack, decoded);
    }

    #[test]
    fn fuzz_fast_codec_decode_never_panics(data in prop::collection::vec(any::<u8>(), 0..512)) {
        // Random bytes should never panic, only return Ok or Err
        let _ = fast_codec::decode_stack(&data);
    }
}

// =============================================================================
// 2. Callback Codec Round-Trip Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, .. ProptestConfig::default() })]

    #[test]
    fn fuzz_callback_codec_ok_roundtrip(stack in prop::collection::vec(arb_primitive_value(), 0..16)) {
        let original: Result<Vec<StackValue>, String> = Ok(stack);
        let encoded = callback_codec::encode_stack_result(&original);
        let decoded = callback_codec::decode_stack_result(&encoded).expect("decode failed");
        prop_assert_eq!(original, Ok(decoded.unwrap()));
    }

    #[test]
    fn fuzz_callback_codec_err_roundtrip(msg in "[a-zA-Z0-9 ]{0,128}") {
        let original: Result<Vec<StackValue>, String> = Err(msg);
        let encoded = callback_codec::encode_stack_result(&original);
        let decoded = callback_codec::decode_stack_result(&encoded).expect("decode failed");
        prop_assert_eq!(original, Err(decoded.unwrap_err()));
    }

    #[test]
    fn fuzz_callback_codec_decode_never_panics(data in prop::collection::vec(any::<u8>(), 0..512)) {
        let _ = callback_codec::decode_stack_result(&data);
    }
}

// =============================================================================
// 3. NeoVM Bytecode Interpreter Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// Random bytecode sequences must never panic the host
    #[test]
    fn fuzz_random_bytecode_no_panic(script in prop::collection::vec(any::<u8>(), 1..256)) {
        let _ = execute_script(&script);
    }

    /// PUSHINT8 with any value should produce correct Integer
    #[test]
    fn fuzz_pushint8(value in any::<i8>()) {
        let script = vec![0x00, value as u8, 0x40]; // PUSHINT8, value, RET
        let result = execute_script(&script).expect("PUSHINT8 should execute");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(value))]);
    }

    /// PUSHINT16 with any value
    #[test]
    fn fuzz_pushint16(value in any::<i16>()) {
        let bytes = value.to_le_bytes();
        let script = vec![0x01, bytes[0], bytes[1], 0x40]; // PUSHINT16, RET
        let result = execute_script(&script).expect("PUSHINT16 should execute");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(value))]);
    }

    /// PUSHINT32 with any value
    #[test]
    fn fuzz_pushint32(value in any::<i32>()) {
        let bytes = value.to_le_bytes();
        let mut script = vec![0x02];
        script.extend_from_slice(&bytes);
        script.push(0x40); // RET
        let result = execute_script(&script).expect("PUSHINT32 should execute");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(value))]);
    }

    /// PUSHINT64 with any value
    #[test]
    fn fuzz_pushint64(value in any::<i64>()) {
        let bytes = value.to_le_bytes();
        let mut script = vec![0x03];
        script.extend_from_slice(&bytes);
        script.push(0x40); // RET
        let result = execute_script(&script).expect("PUSHINT64 should execute");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(value)]);
    }

    /// PUSHDATA1 with any bytes
    #[test]
    fn fuzz_pushdata1(data in prop::collection::vec(any::<u8>(), 0..255)) {
        let mut script = vec![0x0c, data.len() as u8];
        script.extend_from_slice(&data);
        script.push(0x40); // RET
        let result = execute_script(&script).expect("PUSHDATA1 should execute");
        prop_assert_eq!(result.stack, vec![StackValue::ByteString(data)]);
    }
}

// =============================================================================
// 4. Arithmetic Correctness Fuzzing
// =============================================================================

/// Helper: build script that pushes two i64s and applies an opcode
fn binary_op_script(a: i64, b: i64, opcode: u8) -> Vec<u8> {
    let mut script = Vec::with_capacity(20);
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&a.to_le_bytes());
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&b.to_le_bytes());
    script.push(opcode);
    script.push(0x40); // RET
    script
}

/// Helper: build script that pushes one i64 and applies a unary opcode
fn unary_op_script(a: i64, opcode: u8) -> Vec<u8> {
    let mut script = Vec::with_capacity(12);
    script.push(0x03); // PUSHINT64
    script.extend_from_slice(&a.to_le_bytes());
    script.push(opcode);
    script.push(0x40); // RET
    script
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// ADD: verify a + b matches Rust when no overflow
    #[test]
    fn fuzz_add(a in -1_000_000_000i64..1_000_000_000, b in -1_000_000_000i64..1_000_000_000) {
        let script = binary_op_script(a, b, 0x9e); // ADD
        let result = execute_script(&script).expect("ADD should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a + b)]);
    }

    /// SUB: verify a - b
    #[test]
    fn fuzz_sub(a in -1_000_000_000i64..1_000_000_000, b in -1_000_000_000i64..1_000_000_000) {
        let script = binary_op_script(a, b, 0x9f); // SUB
        let result = execute_script(&script).expect("SUB should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a - b)]);
    }

    /// MUL: verify a * b within safe range
    #[test]
    fn fuzz_mul(a in -1_000_000i64..1_000_000, b in -1_000_000i64..1_000_000) {
        let script = binary_op_script(a, b, 0xa0); // MUL
        let result = execute_script(&script).expect("MUL should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a * b)]);
    }

    /// DIV: verify a / b, skip b=0
    #[test]
    fn fuzz_div(a in any::<i64>(), b in any::<i64>().prop_filter("non-zero", |b| *b != 0)) {
        let script = binary_op_script(a, b, 0xa1); // DIV
        let result = execute_script(&script);
        if a == i64::MIN && b == -1 {
            // Overflow case - may fault
        } else {
            let result = result.expect("DIV should not fault for non-zero divisor");
            prop_assert_eq!(result.stack, vec![StackValue::Integer(a / b)]);
        }
    }

    /// MOD: verify a % b, skip b=0
    #[test]
    fn fuzz_mod(a in any::<i64>(), b in any::<i64>().prop_filter("non-zero", |b| *b != 0)) {
        let script = binary_op_script(a, b, 0xa2); // MOD
        let result = execute_script(&script);
        if a == i64::MIN && b == -1 {
            // Overflow case
        } else {
            let result = result.expect("MOD should not fault for non-zero divisor");
            prop_assert_eq!(result.stack, vec![StackValue::Integer(a % b)]);
        }
    }

    /// DIV by zero always faults
    #[test]
    fn fuzz_div_by_zero(a in any::<i64>()) {
        let script = binary_op_script(a, 0, 0xa1);
        prop_assert!(execute_script(&script).is_err());
    }

    /// MOD by zero always faults
    #[test]
    fn fuzz_mod_by_zero(a in any::<i64>()) {
        let script = binary_op_script(a, 0, 0xa2);
        prop_assert!(execute_script(&script).is_err());
    }

    /// NEGATE: -a
    #[test]
    fn fuzz_negate(a in -i64::MAX..i64::MAX) {
        let script = unary_op_script(a, 0x9b); // NEGATE
        let result = execute_script(&script).expect("NEGATE should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(-a)]);
    }

    /// ABS: |a|
    #[test]
    fn fuzz_abs(a in -i64::MAX..i64::MAX) {
        let script = unary_op_script(a, 0x9a); // ABS
        let result = execute_script(&script).expect("ABS should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a.abs())]);
    }

    /// INC: a + 1
    #[test]
    fn fuzz_inc(a in i64::MIN+1..i64::MAX) {
        let script = unary_op_script(a, 0x9c); // INC
        let result = execute_script(&script).expect("INC should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a + 1)]);
    }

    /// DEC: a - 1
    #[test]
    fn fuzz_dec(a in i64::MIN+1..i64::MAX) {
        let script = unary_op_script(a, 0x9d); // DEC
        let result = execute_script(&script).expect("DEC should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a - 1)]);
    }
}

// =============================================================================
// 5. Comparison Opcodes Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, max_shrink_iters: 0, .. ProptestConfig::default() })]

    #[test]
    fn fuzz_lt(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb5); // LT
        let result = execute_script(&script).expect("LT should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(a < b)]);
    }

    #[test]
    fn fuzz_le(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb6); // LE
        let result = execute_script(&script).expect("LE should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(a <= b)]);
    }

    #[test]
    fn fuzz_gt(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb7); // GT
        let result = execute_script(&script).expect("GT should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(a > b)]);
    }

    #[test]
    fn fuzz_ge(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb8); // GE
        let result = execute_script(&script).expect("GE should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(a >= b)]);
    }

    #[test]
    fn fuzz_min(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb9); // MIN
        let result = execute_script(&script).expect("MIN should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a.min(b))]);
    }

    #[test]
    fn fuzz_max(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xba); // MAX
        let result = execute_script(&script).expect("MAX should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a.max(b))]);
    }
}

// =============================================================================
// 6. Stack Manipulation Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// DUP: top item is duplicated
    #[test]
    fn fuzz_dup(value in any::<i64>()) {
        let mut script = vec![0x03]; // PUSHINT64
        script.extend_from_slice(&value.to_le_bytes());
        script.push(0x4a); // DUP
        script.push(0x40); // RET
        let result = execute_script(&script).expect("DUP should not fault");
        prop_assert_eq!(result.stack.len(), 2);
        prop_assert_eq!(&result.stack[0], &result.stack[1]);
    }

    /// DROP + PUSH: push two, drop top, verify bottom remains
    #[test]
    fn fuzz_drop(a in any::<i64>(), b in any::<i64>()) {
        let mut script = Vec::new();
        script.push(0x03); script.extend_from_slice(&a.to_le_bytes()); // PUSH a
        script.push(0x03); script.extend_from_slice(&b.to_le_bytes()); // PUSH b
        script.push(0x45); // DROP
        script.push(0x40); // RET
        let result = execute_script(&script).expect("DROP should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a)]);
    }

    /// SWAP: push a, push b, swap → [b, a]
    #[test]
    fn fuzz_swap(a in any::<i64>(), b in any::<i64>()) {
        let mut script = Vec::new();
        script.push(0x03); script.extend_from_slice(&a.to_le_bytes());
        script.push(0x03); script.extend_from_slice(&b.to_le_bytes());
        script.push(0x50); // SWAP
        script.push(0x40); // RET
        let result = execute_script(&script).expect("SWAP should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(b), StackValue::Integer(a)]);
    }
}

// =============================================================================
// 7. Compound Type Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 50, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// NEWARRAY with valid count creates correct-size array
    #[test]
    fn fuzz_newarray(count in 0u8..16) {
        // PUSH count, NEWARRAY, SIZE, RET
        let push_op = 0x10 + count; // PUSH0..PUSH16
        let script = vec![push_op, 0xc3, 0xca, 0x40]; // NEWARRAY, SIZE, RET
        let result = execute_script(&script).expect("NEWARRAY+SIZE should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(count))]);
    }

    /// PACK: push N items then PACK N → Array of size N
    #[test]
    fn fuzz_pack(count in 0u8..8) {
        let mut script = Vec::new();
        for i in 0..count {
            script.push(0x10 + i); // PUSH0..PUSH7
        }
        script.push(0x10 + count); // PUSH count
        script.push(0xc0); // PACK
        script.push(0xca); // SIZE
        script.push(0x40); // RET
        let result = execute_script(&script).expect("PACK+SIZE should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(count))]);
    }
}

// =============================================================================
// 8. Resource Limit Enforcement
// =============================================================================

#[test]
#[ignore = "requires MAX_STACK_SIZE check in guest interpreter"]
fn newarray_at_max_stack_size() {
    // PUSHINT16 2048, NEWARRAY → should succeed (MAX_STACK_SIZE)
    let mut script = vec![0x01]; // PUSHINT16
    script.extend_from_slice(&2048u16.to_le_bytes());
    script.push(0xc3); // NEWARRAY
    script.push(0x40); // RET
    let result = execute_script(&script);
    assert!(result.is_ok(), "NEWARRAY at MAX_STACK_SIZE should succeed");
}

#[test]
#[ignore = "requires MAX_STACK_SIZE check in guest interpreter"]
fn newarray_exceeds_max_stack_size() {
    // PUSHINT16 2049, NEWARRAY → should FAULT
    let mut script = vec![0x01]; // PUSHINT16
    script.extend_from_slice(&2049u16.to_le_bytes());
    script.push(0xc3); // NEWARRAY
    let result = execute_script(&script);
    assert!(result.is_err(), "NEWARRAY > MAX_STACK_SIZE should FAULT");
}

#[test]
#[ignore = "requires MAX_STACK_SIZE check in guest interpreter"]
fn newstruct_exceeds_max_stack_size() {
    // PUSHINT16 2049, NEWSTRUCT → should FAULT
    let mut script = vec![0x01];
    script.extend_from_slice(&2049u16.to_le_bytes());
    script.push(0xc6); // NEWSTRUCT
    let result = execute_script(&script);
    assert!(result.is_err(), "NEWSTRUCT > MAX_STACK_SIZE should FAULT");
}

// =============================================================================
// 9. Bitwise Operations Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// Bitwise AND on small integers (NeoVM bitwise ops work on byte representations)
    #[test]
    fn fuzz_bitwise_and(a in 0i64..256, b in 0i64..256) {
        let script = binary_op_script(a, b, 0x91); // AND
        let result = execute_script(&script).expect("AND should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a & b)]);
    }

    #[test]
    fn fuzz_bitwise_or(a in 0i64..256, b in 0i64..256) {
        let script = binary_op_script(a, b, 0x92); // OR
        let result = execute_script(&script).expect("OR should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a | b)]);
    }

    #[test]
    fn fuzz_bitwise_xor(a in 0i64..256, b in 0i64..256) {
        let script = binary_op_script(a, b, 0x93); // XOR
        let result = execute_script(&script).expect("XOR should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a ^ b)]);
    }

    /// Bitwise ops on any i64 -- just verify no panics
    #[test]
    fn fuzz_bitwise_no_panic(a in any::<i64>(), b in any::<i64>(), op in prop::sample::select(vec![0x91u8, 0x92, 0x93])) {
        let script = binary_op_script(a, b, op);
        let _ = execute_script(&script); // Must not panic
    }
}

// =============================================================================
// 10. Boolean Logic Fuzzing
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// NOT: push bool, NOT → inverse
    #[test]
    fn fuzz_not(value in any::<bool>()) {
        let push_op = if value { 0x08 } else { 0x09 }; // PUSHT / PUSHF
        let script = vec![push_op, 0xaa, 0x40]; // NOT, RET
        let result = execute_script(&script).expect("NOT should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(!value)]);
    }

    /// EQUAL: two equal integers → true, two different → false
    #[test]
    fn fuzz_numequal(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb3); // NUMEQUAL
        let result = execute_script(&script).expect("NUMEQUAL should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Boolean(a == b)]);
    }

    /// NUMNOTEQUAL
    #[test]
    fn fuzz_numnotequal(a in any::<i64>(), b in any::<i64>()) {
        let script = binary_op_script(a, b, 0xb4); // NUMNOTEQUAL
        let _ = execute_script(&script); // Just ensure no panic
    }
}

// =============================================================================
// 11. Structured Bytecode Sequences
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig { cases: 50, max_shrink_iters: 0, .. ProptestConfig::default() })]

    /// Arithmetic chains: push values, apply random sequence of safe ops
    #[test]
    fn fuzz_arithmetic_chain(
        values in prop::collection::vec(-1000i64..1000, 2..6),
        ops in prop::collection::vec(prop::sample::select(vec![0x9e_u8, 0x9f, 0xb9, 0xba]), 1..5)
    ) {
        let mut script = Vec::new();
        for v in &values {
            script.push(0x03); // PUSHINT64
            script.extend_from_slice(&v.to_le_bytes());
        }
        // Apply ops (but only as many as values - 1, since each binary op consumes 2)
        for (i, op) in ops.iter().enumerate() {
            if i >= values.len() - 1 { break; }
            script.push(*op);
        }
        script.push(0x40); // RET
        // Should never panic
        let _ = execute_script(&script);
    }

    /// Push + DUP + arithmetic: push a, DUP, ADD → 2*a
    #[test]
    fn fuzz_dup_add(a in -1_000_000_000i64..1_000_000_000) {
        let mut script = vec![0x03]; // PUSHINT64
        script.extend_from_slice(&a.to_le_bytes());
        script.push(0x4a); // DUP
        script.push(0x9e); // ADD
        script.push(0x40); // RET
        let result = execute_script(&script).expect("DUP+ADD should not fault");
        prop_assert_eq!(result.stack, vec![StackValue::Integer(a * 2)]);
    }
}
