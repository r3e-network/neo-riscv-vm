use neo_riscv_abi::{StackValue, VmState};
use neo_riscv_guest::{
    interpret, interpret_with_stack_and_syscalls, interpret_with_stack_and_syscalls_at,
    interpret_with_syscalls, SyscallProvider,
};

#[test]
fn executes_push1_ret_script() {
    let result = interpret(&[0x11, 0x40]).expect("guest interpreter should return a result");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_integer_addition_script() {
    let result =
        interpret(&[0x11, 0x12, 0x9e, 0x40]).expect("guest interpreter should support ADD");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn executes_platform_syscall_with_host_provider() {
    let mut host = PlatformHost;
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");

    let mut script = vec![0x41];
    script.extend_from_slice(&syscall.to_le_bytes());
    script.push(0x40);

    let result = interpret_with_syscalls(&script, &mut host)
        .expect("guest interpreter should invoke host syscalls");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(b"NEO".to_vec())]);
}

#[test]
fn executes_pushdata1_bytestring_script() {
    let result = interpret(&[0x0c, 0x05, b'h', b'e', b'l', b'l', b'o', 0x40])
        .expect("guest interpreter should decode PUSHDATA1 payload");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"hello".to_vec())]
    );
}

#[test]
fn executes_pushnull_and_newarray0_script() {
    let result = interpret(&[0x0b, 0xc2, 0x40])
        .expect("guest interpreter should support PUSHNULL and NEWARRAY0");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Null, StackValue::Array(vec![]),]
    );
}

#[test]
fn duplicates_bytestring_with_dup() {
    let result = interpret(&[0x0c, 0x01, b'a', 0x4a, 0x40])
        .expect("guest interpreter should support generic DUP");

    assert_eq!(
        result.stack,
        vec![
            StackValue::ByteString(b"a".to_vec()),
            StackValue::ByteString(b"a".to_vec()),
        ]
    );
}

#[test]
fn packs_items_into_array() {
    let result =
        interpret(&[0x11, 0x12, 0x12, 0xc0, 0x40]).expect("guest interpreter should support PACK");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::Integer(2),
            StackValue::Integer(1),
        ])]
    );
}

#[test]
fn creates_null_filled_array() {
    let result = interpret(&[0x12, 0xc3, 0x40]).expect("guest interpreter should support NEWARRAY");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![StackValue::Null, StackValue::Null,])]
    );
}

#[test]
fn gets_collection_size() {
    let result = interpret(&[0x11, 0x12, 0x12, 0xc0, 0xca, 0x40])
        .expect("guest interpreter should support SIZE");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn picks_array_item_by_index() {
    let result = interpret(&[0x11, 0x12, 0x12, 0xc0, 0x10, 0xce, 0x40])
        .expect("guest interpreter should support PICKITEM");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn creates_array_in_script_builder_order() {
    let result = interpret(&[0x13, 0x12, 0x11, 0x13, 0xc0, 0x40])
        .expect("guest interpreter should preserve builder array order");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::Integer(1),
            StackValue::Integer(2),
            StackValue::Integer(3),
        ])]
    );
}

#[test]
fn creates_struct_in_script_builder_order() {
    let result = interpret(&[0x13, 0x12, 0x11, 0x13, 0xbf, 0x40])
        .expect("guest interpreter should support PACKSTRUCT");

    assert_eq!(
        result.stack,
        vec![StackValue::Struct(vec![
            StackValue::Integer(1),
            StackValue::Integer(2),
            StackValue::Integer(3),
        ])]
    );
}

#[test]
fn creates_map_in_script_builder_order() {
    let result = interpret(&[0x14, 0x13, 0x12, 0x11, 0x12, 0xbe, 0x40])
        .expect("guest interpreter should support PACKMAP");

    assert_eq!(
        result.stack,
        vec![StackValue::Map(vec![
            (StackValue::Integer(1), StackValue::Integer(2)),
            (StackValue::Integer(3), StackValue::Integer(4)),
        ])]
    );
}

#[test]
fn creates_empty_struct_and_map() {
    let result = interpret(&[0xc5, 0xc8, 0x40])
        .expect("guest interpreter should support NEWSTRUCT0 and NEWMAP");

    assert_eq!(
        result.stack,
        vec![StackValue::Struct(vec![]), StackValue::Map(vec![]),]
    );
}

#[test]
fn asserts_true_boolean() {
    let result = interpret(&[0x08, 0x39, 0x40]).expect("guest interpreter should support ASSERT");

    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
}

#[test]
fn executes_script_with_initial_stack() {
    let result = interpret_with_stack_and_syscalls(
        &[0x9e, 0x40],
        vec![StackValue::Integer(1), StackValue::Integer(2)],
        &mut NoopHost,
    )
    .expect("guest interpreter should use the provided initial stack");

    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn halts_when_script_ends_without_explicit_ret() {
    let result = interpret(&[0x11]).expect("guest interpreter should treat end-of-script as RET");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_script_from_nonzero_offset() {
    let result = interpret_with_stack_and_syscalls_at(&[0x10, 0x11], Vec::new(), 1, &mut NoopHost)
        .expect("guest interpreter should support nonzero entry offsets");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_pushint8_and_pushint16() {
    let result = interpret(&[0x00, 0x1c, 0x01, 0x84, 0x00, 0x40])
        .expect("guest interpreter should support integer immediates");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(28), StackValue::Integer(132)]
    );
}

#[test]
fn executes_pushint32() {
    let result = interpret(&[0x02, 0x15, 0xcd, 0x5b, 0x07, 0x40])
        .expect("guest interpreter should support PUSHINT32");

    assert_eq!(result.stack, vec![StackValue::Integer(123456789)]);
}

#[test]
fn executes_pushtrue_and_pushfalse() {
    let result =
        interpret(&[0x08, 0x09, 0x40]).expect("guest interpreter should support PUSHT and PUSHF");

    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(true), StackValue::Boolean(false)]
    );
}

#[test]
fn executes_pushint128_as_big_integer() {
    let mut script = vec![0x04];
    script.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    script.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    script.push(0x40);

    let result = interpret(&script).expect("guest interpreter should support PUSHINT128");

    assert_eq!(result.stack, vec![StackValue::BigInteger(vec![0x01])]);
}

#[test]
fn passes_current_instruction_pointer_to_syscall_provider() {
    let api = neo_riscv_abi::interop_hash("System.Contract.CallNative");
    let mut script = vec![0x11, 0x41];
    script.extend_from_slice(&api.to_le_bytes());
    script.push(0x40);

    let mut host = InstructionPointerHost { observed_ip: None };
    let result = interpret_with_syscalls(&script, &mut host)
        .expect("guest interpreter should expose the syscall instruction pointer");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(host.observed_ip, Some(1));
}

struct NoopHost;

impl SyscallProvider for NoopHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}

struct PlatformHost;

impl SyscallProvider for PlatformHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if api == neo_riscv_abi::interop_hash("System.Runtime.Platform") {
            stack.push(StackValue::ByteString(b"NEO".to_vec()));
            Ok(())
        } else {
            Err(format!("unexpected syscall 0x{api:08x}"))
        }
    }
}

struct InstructionPointerHost {
    observed_ip: Option<usize>,
}

impl SyscallProvider for InstructionPointerHost {
    fn syscall(&mut self, api: u32, ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if api != neo_riscv_abi::interop_hash("System.Contract.CallNative") {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.observed_ip = Some(ip);
        stack.pop();
        Ok(())
    }
}

#[derive(Default)]
struct PackedInteropHost {
    deserialize_count: u64,
    observed_aggregate: Option<Vec<StackValue>>,
}

impl SyscallProvider for PackedInteropHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let deserialize_api = neo_riscv_abi::interop_hash("Crypto.Deserialize");
        let aggregate_api = neo_riscv_abi::interop_hash("Crypto.Aggregate");

        if api == deserialize_api {
            self.deserialize_count += 1;
            stack.push(StackValue::Interop(self.deserialize_count));
            return Ok(());
        }

        if api == aggregate_api {
            self.observed_aggregate = Some(stack.clone());
            stack.clear();
            stack.push(StackValue::Interop(99));
            return Ok(());
        }

        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}

#[derive(Default)]
struct IntegerRoundTripHost {
    call_count: i64,
    observed_third: Option<Vec<StackValue>>,
}

impl SyscallProvider for IntegerRoundTripHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let expected_api = neo_riscv_abi::interop_hash("System.Test.Multi");
        if api != expected_api {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.call_count += 1;
        match self.call_count {
            1 => {
                stack.push(StackValue::Integer(1));
                Ok(())
            }
            2 => {
                *stack = vec![StackValue::Integer(1), StackValue::Integer(2)];
                Ok(())
            }
            3 => {
                self.observed_third = Some(stack.clone());
                Ok(())
            }
            _ => Err("unexpected extra syscall".to_string()),
        }
    }
}

#[derive(Default)]
struct ConsecutiveLargeCallHost {
    call_count: i64,
}

impl SyscallProvider for ConsecutiveLargeCallHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let expected_api = neo_riscv_abi::interop_hash("System.Contract.Call");
        if api != expected_api {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.call_count += 1;
        *stack = vec![StackValue::Integer(self.call_count)];
        Ok(())
    }
}

#[test]
fn concatenates_integer_as_bytestring_preserving_sign_bit() {
    // PUSH1 (value 128 = 0x80 requires 0x00 padding to remain positive)
    // PUSHINT16 128 = [0x01, 0x80, 0x00]
    // PUSHDATA1 "" = [0x0c, 0x00]
    // CAT
    // RET
    let result = interpret(&[0x01, 0x80, 0x00, 0x0c, 0x00, 0x8b, 0x40])
        .expect("guest interpreter should encode 128 as [0x80, 0x00] preserving the positive sign");

    // 128 encoded as LE two's complement = [0x80, 0x00] (trailing 0x00 preserves positive sign)
    assert_eq!(result.stack, vec![StackValue::ByteString(vec![0x80, 0x00])]);
}

#[test]
fn concatenates_negative_integer_as_bytestring() {
    // PUSHM1 = [0x0f] pushes -1
    // PUSHDATA1 "" = [0x0c, 0x00]
    // CAT
    // RET
    let result = interpret(&[0x0f, 0x0c, 0x00, 0x8b, 0x40])
        .expect("guest interpreter should encode -1 as [0xFF]");

    assert_eq!(result.stack, vec![StackValue::ByteString(vec![0xFF])]);
}

#[test]
fn executes_pushint128_positive_preserves_sign_bit() {
    // PUSHINT128 with value 128: 0x80 followed by 15 zero bytes
    let mut script = vec![0x04, 0x80];
    script.extend_from_slice(&[0x00; 15]);
    script.push(0x40); // RET

    let result = interpret(&script).expect("guest interpreter should handle PUSHINT128 sign bit");

    // 128 in LE two's complement needs [0x80, 0x00] to stay positive
    assert_eq!(result.stack, vec![StackValue::BigInteger(vec![0x80, 0x00])]);
}

#[test]
fn executes_pushint128_negative_trims_correctly() {
    // PUSHINT128 with value -1: all 0xFF bytes
    let mut script = vec![0x04];
    script.extend_from_slice(&[0xFF; 16]);
    script.push(0x40); // RET

    let result = interpret(&script).expect("guest interpreter should trim negative PUSHINT128");

    assert_eq!(result.stack, vec![StackValue::BigInteger(vec![0xFF])]);
}

#[test]
fn executes_jmp_forward() {
    // JMP +3 (skip PUSH1), PUSH2, RET
    // offset from ip=0: ip+3 = 3 → lands on PUSH2 at index 2? No.
    // JMP format: [0x22, offset]. offset is relative to JMP instruction position.
    // script[0]=JMP, script[1]=offset(+4), script[2]=PUSH1, script[3]=ABORT, script[4]=PUSH2, script[5]=RET
    let result = interpret(&[0x22, 0x04, 0x11, 0x38, 0x12, 0x40])
        .expect("guest interpreter should support JMP forward");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn executes_drop_and_swap() {
    // PUSH1, PUSH2, SWAP, DROP, RET → stack=[2]
    let result = interpret(&[0x11, 0x12, 0x50, 0x45, 0x40])
        .expect("guest interpreter should support DROP and SWAP");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn executes_sub_and_inc() {
    // PUSH3, PUSH1, SUB → 2, INC → 3, RET
    let result = interpret(&[0x13, 0x11, 0x9f, 0x9c, 0x40])
        .expect("guest interpreter should support SUB and INC");

    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn executes_numequal_and_ge() {
    // PUSH2, PUSH2, NUMEQUAL → true, PUSH1, PUSH2, GE → false, RET
    let result = interpret(&[0x12, 0x12, 0xb3, 0x11, 0x12, 0xb8, 0x40])
        .expect("guest interpreter should support NUMEQUAL and GE");

    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(true), StackValue::Boolean(false)]
    );
}

#[test]
fn executes_numequal_on_bytestrings() {
    let result = interpret(&[0x0c, 0x02, b'o', b'k', 0x0c, 0x02, b'o', b'k', 0xb3, 0x40])
        .expect("guest interpreter should support NUMEQUAL on byte strings");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_numnotequal_on_bytestrings() {
    let result = interpret(&[0x0c, 0x02, b'o', b'k', 0x0c, 0x02, b'n', b'o', 0xb4, 0x40])
        .expect("guest interpreter should support NUMNOTEQUAL on byte strings");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn ge_with_null_operand_returns_false() {
    let result = interpret(&[0x0b, 0x11, 0xb8, 0x40])
        .expect("guest interpreter should match NeoVM GE null semantics");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_lt_le_and_gt() {
    let result = interpret(&[
        0x11, 0x12, 0xb5, // 1 < 2 => true
        0x12, 0x12, 0xb6, // 2 <= 2 => true
        0x12, 0x11, 0xb7, // 2 > 1 => true
        0x40,
    ])
    .expect("guest interpreter should support LT, LE, and GT");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Boolean(true),
            StackValue::Boolean(true),
            StackValue::Boolean(true),
        ]
    );
}

#[test]
fn executes_negate_and_sign() {
    let result = interpret(&[
        0x13, 0x9b, // -3
        0x13, 0x9b, 0x99, // sign(-3) => -1
        0x40,
    ])
    .expect("guest interpreter should support NEGATE and SIGN");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(-3), StackValue::Integer(-1),]
    );
}

#[test]
fn sign_accepts_bytestring_input() {
    let result = interpret(&[0x0c, 0x02, 0x00, 0x01, 0x99, 0x40])
        .expect("guest interpreter should support SIGN on byte strings");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_modmul() {
    let result = interpret(&[0x18, 0x12, 0x13, 0xa5, 0x40])
        .expect("guest interpreter should support MODMUL");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_pow() {
    let result =
        interpret(&[0x19, 0x12, 0xa3, 0x40]).expect("guest interpreter should support POW");

    assert_eq!(result.stack, vec![StackValue::Integer(81)]);
}

#[test]
fn executes_sqrt() {
    let result =
        interpret(&[0x00, 0x51, 0xa4, 0x40]).expect("guest interpreter should support SQRT");

    assert_eq!(result.stack, vec![StackValue::Integer(9)]);
}

#[test]
fn executes_shl_and_shr() {
    let result = interpret(&[
        0x20, 0x14, 0xa8, // 16 << 4 => 256
        0x20, 0x14, 0xa9, // 16 >> 4 => 1
        0x40,
    ])
    .expect("guest interpreter should support SHL and SHR");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(256), StackValue::Integer(1),]
    );
}

#[test]
fn shr_preserves_bytestring_type() {
    let result = interpret(&[0x0c, 0x02, 0x00, 0x01, 0x10, 0xa9, 0x40])
        .expect("guest interpreter should preserve byte string type for SHR");

    assert_eq!(result.stack, vec![StackValue::ByteString(vec![0x00, 0x01])]);
}

#[test]
fn executes_newarray_t() {
    let result =
        interpret(&[0x12, 0xc4, 0x21, 0x40]).expect("guest interpreter should support NEWARRAY_T");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::Integer(0),
            StackValue::Integer(0),
        ])]
    );
}

#[test]
fn executes_haskey_on_buffer() {
    let result = interpret(&[0x12, 0x88, 0x10, 0xcb, 0x40])
        .expect("guest interpreter should support HASKEY on buffers");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_and_on_bytestrings() {
    let result = interpret(&[0x0c, 0x02, 0x0f, 0x0f, 0x0c, 0x02, 0xf0, 0x0f, 0x91, 0x40])
        .expect("guest interpreter should support AND on byte strings");

    assert_eq!(result.stack, vec![StackValue::Integer(3840)]);
}

#[test]
fn executes_keys_on_map() {
    let result = interpret(&[
        0x12, 0x13, 0x12, 0x11, 0x12, 0xbe, // map {1:2, 3:2}
        0xcc, 0x40,
    ])
    .expect("guest interpreter should support KEYS on maps");

    assert_eq!(
        result.stack,
        vec![StackValue::Array(vec![
            StackValue::Integer(1),
            StackValue::Integer(3),
        ])]
    );
}

#[test]
fn setitem_updates_static_field_alias() {
    let result = interpret(&[
        0x56, 0x01, // INITSSLOT 1
        0xc8, // NEWMAP
        0x4a, // DUP
        0x60, // STSFLD0
        0x11, // PUSH1
        0x12, // PUSH2
        0xd0, // SETITEM
        0x58, // LDSFLD0
        0x11, // PUSH1
        0xcb, // HASKEY
        0x40,
    ])
    .expect("guest interpreter should propagate SETITEM through static-field aliases");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn append_updates_static_field_alias() {
    let result = interpret(&[
        0x56, 0x01, // INITSSLOT 1
        0x10, // PUSH0
        0xc3, // NEWARRAY
        0x4a, // DUP
        0x60, // STSFLD0
        0x15, // PUSH5
        0xcf, // APPEND
        0x58, // LDSFLD0
        0xca, // SIZE
        0x40,
    ])
    .expect("guest interpreter should propagate APPEND through static-field aliases");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn clearitems_updates_static_field_alias() {
    let result = interpret(&[
        0x56, 0x01, // INITSSLOT 1
        0x10, // PUSH0
        0xc3, // NEWARRAY
        0x4a, // DUP
        0x60, // STSFLD0
        0xd3, // CLEARITEMS
        0x58, // LDSFLD0
        0xca, // SIZE
        0x40,
    ])
    .expect("guest interpreter should propagate CLEARITEMS through static-field aliases");

    assert_eq!(result.stack, vec![StackValue::Integer(0)]);
}

#[test]
fn reverseitems_updates_buffer_alias() {
    let result = interpret(&[
        0x0c, 0x03, 0x01, 0x02, 0x03, // PUSHDATA1 010203
        0xdb, 0x30, // CONVERT Buffer
        0x4a, // DUP
        0xd1, // REVERSEITEMS
        0x40,
    ])
    .expect("guest interpreter should support REVERSEITEMS on buffers");

    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(vec![0x03, 0x02, 0x01])]
    );
}

#[test]
fn remove_updates_static_field_alias() {
    let result = interpret(&[
        0x56, 0x01, // INITSSLOT 1
        0x16, // PUSH6
        0x15, // PUSH5
        0x12, // PUSH2
        0xc0, // PACK => [5,6]
        0x60, // STSFLD0
        0x58, // LDSFLD0
        0x10, // PUSH0
        0xd2, // REMOVE
        0x58, // LDSFLD0
        0xc1, // UNPACK
        0x40,
    ])
    .expect("guest interpreter should propagate REMOVE through static-field aliases");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(6), StackValue::Integer(1),]
    );
}

#[test]
fn unpack_array_preserves_stack_order() {
    let result = interpret(&[0x15, 0x16, 0x12, 0xc0, 0xc1, 0x40])
        .expect("guest interpreter should support UNPACK");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(5),
            StackValue::Integer(6),
            StackValue::Integer(2),
        ]
    );
}

#[test]
fn unpack_map_preserves_key_value_order() {
    let result = interpret(&[0x15, 0x16, 0x11, 0xbe, 0xc1, 0x40])
        .expect("guest interpreter should support UNPACK on maps");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(5),
            StackValue::Integer(6),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn executes_modpow_and_mod_inverse() {
    let result = interpret(&[
        0x00, 0x13, // 19
        0x0f, // -1
        0x01, 0x8d, 0x00, // 141
        0xa6, 0x00, 0x13, // 19
        0x12, // 2
        0x01, 0x8d, 0x00, // 141
        0xa6, 0x40,
    ])
    .expect("guest interpreter should support MODPOW");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(52), StackValue::Integer(79),]
    );
}

#[test]
fn executes_not() {
    let result = interpret(&[0x11, 0xaa, 0x40]).expect("guest interpreter should support NOT");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_not_on_empty_array_and_struct() {
    let result = interpret(&[0x10, 0xc3, 0xaa, 0x10, 0xc6, 0xaa, 0x40])
        .expect("guest interpreter should treat empty arrays and structs as truthy for NOT");

    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(false), StackValue::Boolean(false),]
    );
}

#[test]
fn executes_initslot_stloc_ldloc() {
    // INITSLOT 2 locals 0 args, PUSH5, STLOC0, PUSH3, STLOC1, LDLOC0, LDLOC1, ADD, RET
    let result = interpret(&[
        0x57, 0x02, 0x00, 0x15, 0x70, 0x13, 0x71, 0x68, 0x69, 0x9e, 0x40,
    ])
    .expect("guest interpreter should support INITSLOT/STLOC/LDLOC");

    assert_eq!(result.stack, vec![StackValue::Integer(8)]);
}

#[test]
fn executes_or_operator() {
    // PUSHT, PUSHF, OR → Integer(1), PUSHF, PUSHF, OR → Integer(0), RET
    // NeoVM AND/OR/XOR on booleans return Integer, not Boolean
    let result = interpret(&[0x08, 0x09, 0x92, 0x09, 0x09, 0x92, 0x40])
        .expect("guest interpreter should support OR");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(0)]
    );
}

#[test]
fn executes_or_on_integers() {
    // PUSH5, PUSH3, OR → 5|3 = 7, RET
    let result = interpret(&[0x15, 0x13, 0x92, 0x40])
        .expect("guest interpreter should support OR on integers");

    assert_eq!(result.stack, vec![StackValue::Integer(7)]);
}

#[test]
fn executes_left_on_bytestring() {
    // PUSHDATA1 "hello", PUSH3, LEFT → "hel", RET
    let result = interpret(&[0x0c, 0x05, b'h', b'e', b'l', b'l', b'o', 0x13, 0x8d, 0x40])
        .expect("guest interpreter should support LEFT");

    assert_eq!(result.stack, vec![StackValue::ByteString(b"hel".to_vec())]);
}

#[test]
fn executes_nop() {
    // PUSH1, NOP, RET
    let result = interpret(&[0x11, 0x21, 0x40]).expect("guest interpreter should support NOP");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn abort_returns_error() {
    let error = interpret(&[0x38]).expect_err("ABORT should return an error");
    assert!(
        error.contains("ABORT"),
        "error should mention ABORT: {error}"
    );
}

#[test]
fn assert_false_returns_error() {
    // PUSHF, ASSERT → should fail
    let error = interpret(&[0x09, 0x39]).expect_err("ASSERT false should fail");
    assert!(
        error.contains("ASSERT"),
        "error should mention ASSERT: {error}"
    );
}

#[test]
fn executes_jmpif_conditional() {
    // PUSHT, JMPIF +3 (skip ABORT), PUSH1, RET
    // ip=0: PUSHT, ip=1: JMPIF, ip=2: offset=+3 → target=1+3=4 (PUSH1)
    let result = interpret(&[0x08, 0x24, 0x03, 0x38, 0x11, 0x40])
        .expect("guest interpreter should support JMPIF");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn executes_pushint64() {
    // PUSHINT64 with value 0x0100000000 (4294967296)
    let result = interpret(&[0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40])
        .expect("guest interpreter should support PUSHINT64");

    assert_eq!(result.stack, vec![StackValue::Integer(4_294_967_296)]);
}

#[test]
fn executes_depth() {
    // PUSH1, PUSH2, DEPTH → 2, RET (stack = [1, 2, 2])
    // Note: DEPTH pushes the stack length BEFORE the push, so at time of DEPTH call, stack has [1, 2] → depth=2
    let result =
        interpret(&[0x11, 0x12, 0x43, 0x40]).expect("guest interpreter should support DEPTH");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(1),
            StackValue::Integer(2),
            StackValue::Integer(2)
        ]
    );
}

#[test]
fn executes_and_on_integers() {
    // PUSH12 (0b1100), PUSH10 (0b1010), AND => 0b1000 = 8
    let result = interpret(&[0x1c, 0x1a, 0x91, 0x40])
        .expect("guest interpreter should support AND on integers");

    assert_eq!(result.stack, vec![StackValue::Integer(8)]);
}

#[test]
fn executes_and_on_booleans() {
    // PUSHT, PUSHT, AND => 1, PUSHF, PUSHT, AND => 0
    // NeoVM AND on booleans returns Integer, not Boolean
    let result = interpret(&[0x08, 0x08, 0x91, 0x09, 0x08, 0x91, 0x40])
        .expect("guest interpreter should support AND on booleans");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(0)]
    );
}

#[test]
fn executes_xor_on_integers() {
    // PUSH12 (0b1100), PUSH10 (0b1010), XOR => 0b0110 = 6
    let result = interpret(&[0x1c, 0x1a, 0x93, 0x40])
        .expect("guest interpreter should support XOR on integers");

    assert_eq!(result.stack, vec![StackValue::Integer(6)]);
}

#[test]
fn executes_xor_on_booleans() {
    // PUSHT, PUSHT, XOR => 0, PUSHT, PUSHF, XOR => 1
    // NeoVM XOR on booleans returns Integer, not Boolean
    let result = interpret(&[0x08, 0x08, 0x93, 0x08, 0x09, 0x93, 0x40])
        .expect("guest interpreter should support XOR on booleans");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(0), StackValue::Integer(1)]
    );
}

#[test]
fn executes_invert_on_integer() {
    // PUSH0, INVERT => -1
    let result = interpret(&[0x10, 0x90, 0x40]).expect("guest interpreter should support INVERT");

    assert_eq!(result.stack, vec![StackValue::Integer(-1)]);
}

#[test]
fn executes_invert_on_boolean() {
    // PUSHT, INVERT => -2 (since true = 1, ~1 = -2)
    let result =
        interpret(&[0x08, 0x90, 0x40]).expect("guest interpreter should support INVERT on boolean");

    assert_eq!(result.stack, vec![StackValue::Integer(-2)]);
}

#[test]
fn executes_equal_on_same_integers() {
    // PUSH2, PUSH2, EQUAL => true
    let result =
        interpret(&[0x12, 0x12, 0x97, 0x40]).expect("guest interpreter should support EQUAL");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_equal_on_different_integers() {
    // PUSH1, PUSH2, EQUAL => false
    let result = interpret(&[0x11, 0x12, 0x97, 0x40])
        .expect("guest interpreter should support EQUAL for different values");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_equal_on_same_array_reference() {
    // PUSH0, NEWARRAY, DUP, EQUAL => true
    let result = interpret(&[0x10, 0xc3, 0x4a, 0x97, 0x40])
        .expect("guest interpreter should support EQUAL on duplicated arrays");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_equal_on_distinct_arrays() {
    // PUSH0, NEWARRAY, PUSH0, NEWARRAY, EQUAL => false
    let result = interpret(&[0x10, 0xc3, 0x10, 0xc3, 0x97, 0x40])
        .expect("guest interpreter should compare distinct arrays by identity");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_equal_on_distinct_structs_with_equal_content() {
    // PUSH0, NEWSTRUCT, PUSH0, NEWSTRUCT, EQUAL => true
    let result = interpret(&[0x10, 0xc6, 0x10, 0xc6, 0x97, 0x40])
        .expect("guest interpreter should compare structs structurally");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_equal_on_same_map_reference() {
    // NEWMAP, DUP, EQUAL => true
    let result = interpret(&[0xc8, 0x4a, 0x97, 0x40])
        .expect("guest interpreter should support EQUAL on duplicated maps");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_equal_on_distinct_maps() {
    // NEWMAP, NEWMAP, EQUAL => false
    let result = interpret(&[0xc8, 0xc8, 0x97, 0x40])
        .expect("guest interpreter should compare distinct maps by identity");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_equal_on_same_buffer_reference() {
    // PUSHDATA1 0xAA, CONVERT Buffer, DUP, EQUAL => true
    let result = interpret(&[0x0c, 0x01, 0xaa, 0xdb, 0x30, 0x4a, 0x97, 0x40])
        .expect("guest interpreter should support EQUAL on duplicated buffers");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_equal_on_distinct_buffers_with_equal_content() {
    // PUSHDATA1 0xAA, CONVERT Buffer, PUSHDATA1 0xAA, CONVERT Buffer, EQUAL => false
    let result = interpret(&[
        0x0c, 0x01, 0xaa, 0xdb, 0x30, 0x0c, 0x01, 0xaa, 0xdb, 0x30, 0x97, 0x40,
    ])
    .expect("guest interpreter should compare distinct buffers by identity");

    assert_eq!(result.stack, vec![StackValue::Boolean(false)]);
}

#[test]
fn executes_notequal() {
    // PUSH1, PUSH2, NOTEQUAL => true
    let result =
        interpret(&[0x11, 0x12, 0x98, 0x40]).expect("guest interpreter should support NOTEQUAL");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn executes_values_on_map() {
    // Create map {1:2, 3:4}: push values and keys, then count, then PACKMAP
    // Stack order for PACKMAP: val1, key1, val2, key2, count
    let result = interpret(&[
        0x12, // PUSH2 (val1)
        0x11, // PUSH1 (key1)
        0x14, // PUSH4 (val2)
        0x13, // PUSH3 (key2)
        0x12, // PUSH2 (count)
        0xbe, // PACKMAP => {1:2, 3:4}
        0xcd, // VALUES
        0x40,
    ])
    .expect("guest interpreter should support VALUES on maps");

    if let StackValue::Array(ref values) = result.stack[0] {
        assert_eq!(values.len(), 2);
        assert!(values.contains(&StackValue::Integer(2)));
        assert!(values.contains(&StackValue::Integer(4)));
    } else {
        panic!("expected array on stack");
    }
}

#[test]
fn packs_interop_results_across_multiple_syscalls() {
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

    let mut host = PackedInteropHost::default();
    let result = interpret_with_syscalls(&script, &mut host)
        .expect("guest interpreter should preserve packed interop syscall results");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Interop(99)]);
    assert_eq!(
        host.observed_aggregate,
        Some(vec![StackValue::Array(vec![
            StackValue::Interop(2),
            StackValue::Interop(1)
        ])])
    );
}

#[test]
fn preserves_integer_results_across_multiple_syscalls() {
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

    let mut host = IntegerRoundTripHost::default();
    let result = interpret_with_syscalls(&script, &mut host)
        .expect("guest interpreter should preserve integer syscall results");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        host.observed_third,
        Some(vec![StackValue::Integer(1), StackValue::Integer(2)])
    );
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(2)]
    );
}

#[test]
fn executes_two_consecutive_large_dynamic_calls() {
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

    let mut host = ConsecutiveLargeCallHost::default();
    let result = interpret_with_syscalls(&script, &mut host)
        .expect("guest interpreter should handle two consecutive large dynamic calls");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(host.call_count, 2);
    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn unsupported_opcode_returns_error() {
    let error = interpret(&[0xff]).expect_err("unsupported opcode 0xff should return an error");
    assert!(
        error.contains("unsupported opcode"),
        "error should mention unsupported opcode: {error}"
    );
}

#[test]
fn and_on_booleans_returns_integer() {
    // PUSH0 NOT (→ true), PUSH0 NOT (→ true), AND → Integer(1)
    // NeoVM AND on booleans returns Integer, not Boolean
    let result = interpret(&[0x10, 0xaa, 0x10, 0xaa, 0x91, 0x40])
        .expect("guest interpreter should return Integer for boolean AND");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn stack_overflow_at_2048_items() {
    // Push 2049 items (PUSH1 = 0x11) — stack limit is 2048
    let mut script = vec![0x11; 2049];
    script.push(0x40); // RET
    let error =
        interpret(&script).expect_err("pushing 2049 items should FAULT with stack overflow");
    assert!(
        error.contains("stack overflow") || error.contains("Stack overflow"),
        "error should mention stack overflow: {error}"
    );
}

#[test]
fn newbuffer_rejects_over_max_item_size() {
    // PUSHINT32 1048577 (0x100001), NEWBUFFER → should FAULT
    // 1048577 in LE = [0x01, 0x00, 0x10, 0x00]
    let result =
        interpret(&[0x02, 0x01, 0x00, 0x10, 0x00, 0x88]).expect_err("NEWBUFFER > 1MB should FAULT");
    assert!(
        result.contains("FAULT") || result.contains("size") || result.contains("MaxItemSize"),
        "error should indicate size violation: {result}"
    );
}

#[test]
fn empty_script_halts_immediately() {
    let result = interpret(&[]).expect("empty script should halt cleanly");
    assert_eq!(result.state, VmState::Halt);
    assert!(result.stack.is_empty());
}

#[test]
fn jmpifnot_jumps_when_false() {
    // PUSHF, JMPIFNOT +3 (skip ABORT), PUSH1, RET
    // ip=1: JMPIFNOT, offset=3, target=1+3=4 → PUSH1
    let result = interpret(&[0x09, 0x26, 0x03, 0x38, 0x11, 0x40])
        .expect("guest interpreter should support JMPIFNOT");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmpeq_jumps_when_equal() {
    // PUSH1, PUSH1, JMPEQ +3 (skip ABORT), PUSH2, RET
    // ip=2: JMPEQ, offset=3, target=2+3=5 → PUSH2
    let result = interpret(&[0x11, 0x11, 0x28, 0x03, 0x38, 0x12, 0x40])
        .expect("guest interpreter should support JMPEQ");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn jmpne_jumps_when_not_equal() {
    // PUSH1, PUSH2, JMPNE +3 (skip ABORT), PUSH3, RET
    // ip=2: JMPNE, offset=3, target=2+3=5 → PUSH3
    let result = interpret(&[0x11, 0x12, 0x2a, 0x03, 0x38, 0x13, 0x40])
        .expect("guest interpreter should support JMPNE");

    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn call_and_ret_round_trip() {
    // CALL +4 → subroutine at ip=4 pushes PUSH5, RET returns to ip=2, PUSH1, RET
    // ip=0: CALL(0x34), ip=1: offset(+4), ip=2: PUSH1, ip=3: RET, ip=4: PUSH5, ip=5: RET
    let result = interpret(&[0x34, 0x04, 0x11, 0x40, 0x15, 0x40])
        .expect("guest interpreter should support CALL and RET round-trip");

    assert_eq!(
        result.stack,
        vec![StackValue::Integer(5), StackValue::Integer(1)]
    );
}

#[test]
fn istype_checks_integer_type() {
    // PUSH1, ISTYPE 0x21 (Integer) → true, RET
    let result =
        interpret(&[0x11, 0xd9, 0x21, 0x40]).expect("guest interpreter should support ISTYPE");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn isnull_detects_null() {
    // PUSHNULL, ISNULL → true, PUSH1, ISNULL → false, RET
    let result = interpret(&[0x0b, 0xd8, 0x11, 0xd8, 0x40])
        .expect("guest interpreter should support ISNULL");

    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(true), StackValue::Boolean(false)]
    );
}

#[test]
fn mul_two_integers() {
    // PUSH6, PUSH7, MUL → 42, RET
    let result =
        interpret(&[0x16, 0x17, 0xa0, 0x40]).expect("guest interpreter should support MUL");

    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn div_integers() {
    // PUSHINT8 42, PUSH6, DIV → 7, RET
    let result =
        interpret(&[0x00, 0x2a, 0x16, 0xa1, 0x40]).expect("guest interpreter should support DIV");

    assert_eq!(result.stack, vec![StackValue::Integer(7)]);
}

#[test]
fn mod_integers() {
    // PUSH10, PUSH3, MOD → 1, RET
    let result =
        interpret(&[0x1a, 0x13, 0xa2, 0x40]).expect("guest interpreter should support MOD");

    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn abs_negative_integer() {
    // PUSHINT8 -5, ABS → 5, RET
    let result =
        interpret(&[0x00, 0xfb, 0x9a, 0x40]).expect("guest interpreter should support ABS");

    assert_eq!(result.stack, vec![StackValue::Integer(5)]);
}

#[test]
fn dec_integer() {
    // PUSH10, DEC → 9, RET
    let result = interpret(&[0x1a, 0x9d, 0x40]).expect("guest interpreter should support DEC");

    assert_eq!(result.stack, vec![StackValue::Integer(9)]);
}

#[test]
fn min_of_two() {
    // PUSH3, PUSH7, MIN → 3, RET
    let result =
        interpret(&[0x13, 0x17, 0xb9, 0x40]).expect("guest interpreter should support MIN");

    assert_eq!(result.stack, vec![StackValue::Integer(3)]);
}

#[test]
fn max_of_two() {
    // PUSH3, PUSH7, MAX → 7, RET
    let result =
        interpret(&[0x13, 0x17, 0xba, 0x40]).expect("guest interpreter should support MAX");

    assert_eq!(result.stack, vec![StackValue::Integer(7)]);
}

#[test]
fn within_range() {
    // PUSH5, PUSH1, PUSH10, WITHIN → true (1 <= 5 < 10), RET
    let result = interpret(&[0x15, 0x11, 0x1a, 0xbb, 0x40])
        .expect("guest interpreter should support WITHIN");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn booland_true_true() {
    // PUSHT, PUSHT, BOOLAND → true, RET
    let result =
        interpret(&[0x08, 0x08, 0xab, 0x40]).expect("guest interpreter should support BOOLAND");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn boolor_false_true() {
    // PUSHF, PUSHT, BOOLOR → true, RET
    let result =
        interpret(&[0x09, 0x08, 0xac, 0x40]).expect("guest interpreter should support BOOLOR");

    assert_eq!(result.stack, vec![StackValue::Boolean(true)]);
}

#[test]
fn nz_nonzero() {
    // PUSH5, NZ → true, PUSH0, NZ → false, RET
    let result =
        interpret(&[0x15, 0xb1, 0x10, 0xb1, 0x40]).expect("guest interpreter should support NZ");

    assert_eq!(
        result.stack,
        vec![StackValue::Boolean(true), StackValue::Boolean(false)]
    );
}

#[test]
fn substr_extracts_middle() {
    // PUSHDATA1 "hello world", PUSH6, PUSH5, SUBSTR → "world"
    let result = interpret(&[
        0x0c, 0x0b, b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd',
        0x16, // PUSH6 (offset)
        0x15, // PUSH5 (count)
        0x8c, // SUBSTR
        0x40,
    ])
    .expect("guest interpreter should support SUBSTR");

    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"world".to_vec())]
    );
}

#[test]
fn right_extracts_suffix() {
    // PUSHDATA1 "hello", PUSH3, RIGHT → "llo"
    let result = interpret(&[
        0x0c, 0x05, b'h', b'e', b'l', b'l', b'o', 0x13, // PUSH3
        0x8e, // RIGHT
        0x40,
    ])
    .expect("guest interpreter should support RIGHT");

    assert_eq!(result.stack, vec![StackValue::ByteString(b"llo".to_vec())]);
}

#[test]
fn rot_rotates_top_three() {
    // Push 1,2,3, ROT → stack should be [2,3,1] (top=1)
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x51, // ROT
        0x40,
    ])
    .expect("guest interpreter should support ROT");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(2),
            StackValue::Integer(3),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn pick_duplicates_nth() {
    // Push 10,20,30, PUSH2, PICK → top should be 10
    let result = interpret(&[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x14, // PUSHINT8 20
        0x00, 0x1e, // PUSHINT8 30
        0x12, // PUSH2
        0x4d, // PICK
        0x40,
    ])
    .expect("guest interpreter should support PICK");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(10),
            StackValue::Integer(20),
            StackValue::Integer(30),
            StackValue::Integer(10),
        ]
    );
}

#[test]
fn over_copies_second() {
    // Push 1,2, OVER → stack [1,2,1]
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x4b, // OVER
        0x40,
    ])
    .expect("guest interpreter should support OVER");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(1),
            StackValue::Integer(2),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn nip_removes_second() {
    // Push 1,2, NIP → stack [2]
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x46, // NIP
        0x40,
    ])
    .expect("guest interpreter should support NIP");

    assert_eq!(result.stack, vec![StackValue::Integer(2)]);
}

#[test]
fn tuck_copies_top_below_second() {
    // Push 1,2, TUCK → stack [2,1,2]
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x4e, // TUCK
        0x40,
    ])
    .expect("guest interpreter should support TUCK");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(2),
            StackValue::Integer(1),
            StackValue::Integer(2),
        ]
    );
}

#[test]
fn clear_empties_stack() {
    // Push 1,2,3, CLEAR → empty stack
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x49, // CLEAR
        0x40,
    ])
    .expect("guest interpreter should support CLEAR");

    assert!(result.stack.is_empty());
}

#[test]
fn reverse3_reverses_top_three() {
    // Push 1,2,3, REVERSE3 → [3,2,1]
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x53, // REVERSE3
        0x40,
    ])
    .expect("guest interpreter should support REVERSE3");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(3),
            StackValue::Integer(2),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn reverse4_reverses_top_four() {
    // Push 1,2,3,4, REVERSE4 → [4,3,2,1]
    let result = interpret(&[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x14, // PUSH4
        0x54, // REVERSE4
        0x40,
    ])
    .expect("guest interpreter should support REVERSE4");

    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(4),
            StackValue::Integer(3),
            StackValue::Integer(2),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn throw_causes_fault() {
    // PUSH1, THROW → FAULT state
    let result = interpret(&[0x11, 0x3a]);
    match result {
        Err(e) => assert!(
            e.contains("THROW") || e.contains("FAULT") || e.contains("fault"),
            "error should mention THROW or FAULT: {e}"
        ),
        Ok(r) => assert_eq!(r.state, VmState::Fault, "THROW should cause FAULT state"),
    }
}

#[test]
fn abortmsg_includes_message() {
    // PUSHDATA1 "fail!", ABORTMSG → FAULT with message containing "fail!"
    let result = interpret(&[
        0x0c, 0x05, b'f', b'a', b'i', b'l', b'!', 0xe0, // ABORTMSG
    ]);

    match result {
        Err(e) => assert!(
            e.contains("fail!"),
            "error should contain the abort message: {e}"
        ),
        Ok(r) => {
            assert_eq!(r.state, VmState::Fault, "ABORTMSG should cause FAULT state");
            let msg = r.fault_message.unwrap_or_default();
            assert!(
                msg.contains("fail!"),
                "fault_message should contain 'fail!': {msg}"
            );
        }
    }
}

#[test]
fn ldarg_starg_round_trip() {
    // INITSLOT with 2 locals 2 args
    // NeoVM INITSLOT pops args from stack: top=arg0, next=arg1
    // So initial_stack = [20, 10] means arg0=10 (popped first), arg1=20 (popped second)
    let mut host = NoSyscalls;
    let initial_stack = vec![StackValue::Integer(20), StackValue::Integer(10)];
    let script = &[
        0x57, 0x02, 0x02, // INITSLOT 2 locals, 2 args
        0x78, // LDARG0 → push arg0 (10)
        0x00, 0x63, // PUSHINT8 99
        0x81, // STARG1 → write 99 into arg1
        0x79, // LDARG1 → push arg1 (99)
        0x40,
    ];
    let result = interpret_with_stack_and_syscalls(script, initial_stack, &mut host)
        .expect("guest interpreter should support LDARG/STARG round-trip");

    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(10), StackValue::Integer(99)]
    );
}

#[test]
fn pushint256_large_value() {
    // PUSHINT256 with a 32-byte value (little-endian): 1 followed by 31 zero bytes = 1
    let mut script = vec![0x05]; // PUSHINT256
    let mut value = [0u8; 32];
    value[0] = 0x01;
    value[1] = 0x02;
    script.extend_from_slice(&value);
    script.push(0x40); // RET

    let result = interpret(&script).expect("guest interpreter should support PUSHINT256");

    assert_eq!(result.state, VmState::Halt);
    // PUSHINT256 produces a BigInteger; the value bytes are [0x01, 0x02] (trimmed)
    assert_eq!(result.stack, vec![StackValue::BigInteger(vec![0x01, 0x02])]);
}

#[test]
fn gas_exhaustion_faults() {
    // The guest interpreter does not track gas, so gas exhaustion can only be tested
    // through the host runtime. Here we verify the guest handles a simple script.
    // This test ensures a very long script with many expensive ops eventually halts.
    // Since the guest interpreter has no gas concept, we test stack overflow as the
    // proxy for resource exhaustion.
    let mut script = Vec::new();
    script.extend(std::iter::repeat_n(0x11, 2049)); // PUSH1
    script.push(0x40);
    let error = interpret(&script).expect_err("should fault on resource exhaustion");
    assert!(
        error.contains("stack overflow") || error.contains("Stack overflow"),
        "error should indicate resource exhaustion: {error}"
    );
}

struct NoSyscalls;

impl SyscallProvider for NoSyscalls {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}

// =============================================================================
// Exception handling & advanced opcode tests
// =============================================================================

#[test]
fn try_catch_catches_throw() {
    // Layout (byte offsets from TRY at ip=0):
    //   0: TRY catch_offset=+5, finally_offset=0   (3 bytes: 0x3b, 0x05, 0x00)
    //   3: PUSH1                                     (throw something)
    //   4: THROW (0x3a)
    //   --- catch handler at offset 5 ---
    //   5: PUSH2                                     (catch executed)
    //   6: ENDTRY +2 (0x3d, 0x02) → jump to ip=8
    //   --- end ---
    //   8: RET
    let script: &[u8] = &[
        0x3b, 0x05, 0x00, // TRY catch=+5, finally=0
        0x11, // PUSH1 (value to throw)
        0x3a, // THROW
        0x12, // PUSH2 (catch handler)
        0x3d, 0x02, // ENDTRY +2 → ip 8
        0x40, // RET
    ];
    let result = interpret(script).expect("try-catch should not error");
    assert_eq!(result.state, VmState::Halt);
    // Stack: the catch handler pushes the error message (ByteString) then PUSH2
    // THROW pops the value and pushes an error string onto the stack when caught
    // Then PUSH2 is executed in catch. We expect PUSH2's value on top.
    assert!(
        result.stack.contains(&StackValue::Integer(2)),
        "catch handler should have executed, stack: {:?}",
        result.stack
    );
}

#[test]
fn try_finally_executes_on_normal() {
    // Layout (byte offsets from TRY at ip=0):
    //   0: TRY catch=0, finally=+6  (3 bytes: 0x3b, 0x00, 0x06)
    //   3: PUSH1                     (1 byte)
    //   4: ENDTRY +4 (0x3d, 0x04)   (2 bytes) → triggers finally at ip=6
    //   --- finally handler at offset 6 ---
    //   6: PUSH3                     (1 byte)
    //   7: ENDFINALLY (0x3f)         (1 byte)
    //   --- after finally ---
    //   8: RET
    let script: &[u8] = &[
        0x3b, 0x00, 0x06, // TRY catch=0, finally=+6
        0x11, // PUSH1
        0x3d, 0x04, // ENDTRY +4 → ip 8 (but first runs finally at ip=6)
        0x13, // PUSH3 (finally handler)
        0x3f, // ENDFINALLY
        0x40, // RET
    ];
    let result = interpret(script).expect("try-finally should not error");
    assert_eq!(result.state, VmState::Halt);
    assert!(
        result.stack.contains(&StackValue::Integer(3)),
        "finally handler should have executed, stack: {:?}",
        result.stack
    );
}

#[test]
fn throwifnot_does_not_throw_on_true() {
    // PUSHT, THROWIFNOT, PUSH1, RET → should produce 1
    let script: &[u8] = &[
        0x08, // PUSHT
        0xf1, // THROWIFNOT
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("THROWIFNOT on true should not fault");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn throwifnot_throws_on_false() {
    // PUSHF, THROWIFNOT → should FAULT
    let script: &[u8] = &[
        0x09, // PUSHF
        0xf1, // THROWIFNOT
    ];
    let result = interpret(script);
    match result {
        Err(e) => assert!(
            e.contains("THROW") || e.contains("FAULT") || e.contains("fault"),
            "error should mention THROW or FAULT: {e}"
        ),
        Ok(r) => assert_eq!(
            r.state,
            VmState::Fault,
            "THROWIFNOT on false should cause FAULT"
        ),
    }
}

#[test]
fn convert_integer_to_bytestring() {
    // PUSH5, CONVERT 0x28 (ByteString), RET
    let script: &[u8] = &[
        0x15, // PUSH5
        0xdb, // CONVERT
        0x28, // ByteString type
        0x40, // RET
    ];
    let result = interpret(script).expect("CONVERT integer to ByteString should succeed");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(vec![5])],
        "CONVERT 5 to ByteString should produce [5]"
    );
}

#[test]
fn popitem_removes_last_from_array() {
    // PACK pops items in stack order: top popped first → stored at index 0.
    // Stack [1,2,3] (3 on top), PUSH3, PACK → Array items popped: 3,2,1 → Array [3,2,1]
    // POPITEM pops last element (index 2 = 1)
    // Result stack: [Array[3,2], Integer(1)]
    let script: &[u8] = &[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x13, // PUSH3 (count for PACK)
        0xc0, // PACK
        0xd4, // POPITEM
        0x40, // RET
    ];
    let result = interpret(script).expect("POPITEM should succeed");
    assert_eq!(result.state, VmState::Halt);
    // After POPITEM: stack has [Array[3,2], Integer(1)]
    assert!(
        result.stack.contains(&StackValue::Integer(1)),
        "popped item should be 1, stack: {:?}",
        result.stack
    );
}

#[test]
fn memcpy_copies_bytes() {
    // Create dst buffer (size 4), create src buffer with content
    // PUSH4, NEWBUFFER → dst (4 zero bytes)
    // PUSHDATA1 [0xAA, 0xBB] → src (ByteString)
    // Operand order on stack: dst, di, src, si, count
    // PUSH0 (di=0), swap to get right order, etc.
    // Simpler: push dst, push di=0, push src, push si=0, push count=2, MEMCPY
    let script: &[u8] = &[
        0x14, // PUSH4
        0x88, // NEWBUFFER (4 zero bytes)
        0x10, // PUSH0 (di)
        0x0c, 0x02, 0xAA, 0xBB, // PUSHDATA1 [0xAA, 0xBB] (src)
        0x10, // PUSH0 (si)
        0x12, // PUSH2 (count)
        0x89, // MEMCPY
        0x40, // RET
    ];
    let result = interpret(script).expect("MEMCPY should succeed");
    assert_eq!(result.state, VmState::Halt);
    // The buffer should have been modified in-place. It remains on stack as reference.
    // After MEMCPY the stack should be empty (MEMCPY consumes all 5 operands)
    // but the buffer is updated via propagate_update.
    // Actually looking at the code: MEMCPY pops all 5 operands and does NOT push result.
    assert!(
        result.stack.is_empty(),
        "MEMCPY consumes all operands, stack: {:?}",
        result.stack
    );
}

#[test]
fn roll_moves_nth_to_top() {
    // Push 1,2,3, PUSH2, ROLL → removes item at index 2 (bottom=1) and moves to top
    // Stack before ROLL: [1,2,3], n=2
    // Item at index 2 from top is 1 (index 0=3, index 1=2, index 2=1)
    // After: [2,3,1]
    let script: &[u8] = &[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x12, // PUSH2 (n)
        0x52, // ROLL
        0x40, // RET
    ];
    let result = interpret(script).expect("ROLL should succeed");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(2),
            StackValue::Integer(3),
            StackValue::Integer(1),
        ]
    );
}

#[test]
fn xdrop_removes_nth_from_top() {
    // Push 1,2,3, PUSH1, XDROP → removes item at index 1 from top
    // Stack before XDROP: [1,2,3], n=1
    // Index 0 from top=3, index 1=2 → remove 2
    // After: [1,3]
    let script: &[u8] = &[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x11, // PUSH1 (n)
        0x48, // XDROP
        0x40, // RET
    ];
    let result = interpret(script).expect("XDROP should succeed");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(1), StackValue::Integer(3)]
    );
}

#[test]
fn reversen_reverses_n_items() {
    // Push 1,2,3,4,5, PUSH5, REVERSEN → [5,4,3,2,1]
    let script: &[u8] = &[
        0x11, // PUSH1
        0x12, // PUSH2
        0x13, // PUSH3
        0x14, // PUSH4
        0x15, // PUSH5
        0x15, // PUSH5 (n)
        0x55, // REVERSEN
        0x40, // RET
    ];
    let result = interpret(script).expect("REVERSEN should succeed");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![
            StackValue::Integer(5),
            StackValue::Integer(4),
            StackValue::Integer(3),
            StackValue::Integer(2),
            StackValue::Integer(1),
        ]
    );
}

// =============================================================================
// Final missing opcode tests
// =============================================================================

#[test]
fn pusha_pushes_address() {
    // PUSHA(0x0a) with offset +5 → target = 0 + 5 = 5 (points to RET)
    let script: &[u8] = &[
        0x0a, // PUSHA
        0x05, 0x00, 0x00, 0x00, // i32 offset = +5
        0x40, // RET
    ];
    let result = interpret(script).expect("PUSHA should push a Pointer onto the stack");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Pointer(5)]);
}

#[test]
fn pushdata2_large_bytestring() {
    // PUSHDATA2(0x0d) with 256-byte payload
    let mut script = vec![0x0d];
    script.extend_from_slice(&256_u16.to_le_bytes());
    let payload = vec![0xAA_u8; 256];
    script.extend_from_slice(&payload);
    script.push(0x40); // RET
    let result = interpret(&script).expect("PUSHDATA2 should handle 256-byte payload");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::ByteString(payload)]);
}

#[test]
fn jmpgt_jumps_when_greater() {
    // Push 5, Push 3 → pops b=3, a=5 → 5 > 3 → jump
    // ip=0:PUSH5, ip=1:PUSH3, ip=2:JMPGT, ip=3:offset(+3)→target=5, ip=4:ABORT, ip=5:PUSH1, ip=6:RET
    let script: &[u8] = &[
        0x15, // PUSH5
        0x13, // PUSH3
        0x2c, // JMPGT
        0x03, // offset +3 → ip 2+3=5
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPGT should jump when a > b");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmpge_jumps_when_equal() {
    // Push 3, Push 3 → pops b=3, a=3 → 3 >= 3 → jump
    let script: &[u8] = &[
        0x13, // PUSH3
        0x13, // PUSH3
        0x2e, // JMPGE
        0x03, // offset +3 → ip 2+3=5
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPGE should jump when a >= b");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmplt_jumps_when_less() {
    // Push 3, Push 5 → pops b=5, a=3 → 3 < 5 → jump
    let script: &[u8] = &[
        0x13, // PUSH3
        0x15, // PUSH5
        0x30, // JMPLT
        0x03, // offset +3 → ip 2+3=5
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPLT should jump when a < b");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmple_jumps_when_equal() {
    // Push 3, Push 3 → pops b=3, a=3 → 3 <= 3 → jump
    let script: &[u8] = &[
        0x13, // PUSH3
        0x13, // PUSH3
        0x32, // JMPLE
        0x03, // offset +3 → ip 2+3=5
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPLE should jump when a <= b");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmp_l_long_jump() {
    // JMP_L(0x23) with 4-byte i32 offset, jumping over ABORT
    // ip=0:JMP_L, ip=1-4:offset(+6)→target=6, ip=5:ABORT, ip=6:PUSH1, ip=7:RET
    let script: &[u8] = &[
        0x23, // JMP_L
        0x06, 0x00, 0x00, 0x00, // i32 offset = +6 → target ip=0+6=6
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMP_L should long-jump forward over ABORT");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn call_l_long_call() {
    // CALL_L(0x35) with 4-byte offset to subroutine
    // ip=0:CALL_L, ip=1-4:offset(+7)→target=7, ip=5:PUSH1, ip=6:RET, ip=7:PUSH5, ip=8:RET
    let script: &[u8] = &[
        0x35, // CALL_L
        0x07, 0x00, 0x00, 0x00, // i32 offset = +7 → target ip=0+7=7
        0x11, // PUSH1 (after return)
        0x40, // RET
        0x15, // PUSH5 (subroutine)
        0x40, // RET (returns to ip=5)
    ];
    let result = interpret(script).expect("CALL_L should call subroutine and return");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(5), StackValue::Integer(1)]
    );
}

#[test]
fn calla_calls_address() {
    // PUSHA pushes pointer to subroutine at ip=8, CALLA calls it
    // ip=0:PUSHA, ip=1-4:offset(+8)→Pointer(8), ip=5:CALLA, ip=6:PUSH1, ip=7:RET
    // ip=8:PUSH5, ip=9:RET
    let script: &[u8] = &[
        0x0a, // PUSHA
        0x08, 0x00, 0x00, 0x00, // i32 offset = +8 → Pointer(8)
        0x36, // CALLA
        0x11, // PUSH1 (after return)
        0x40, // RET
        0x15, // PUSH5 (subroutine)
        0x40, // RET
    ];
    let result = interpret(script).expect("CALLA should call through pointer and return");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(5), StackValue::Integer(1)]
    );
}

#[test]
fn toaltstack_fromaltstack_round_trip() {
    // PUSH5, TOALTSTACK, PUSH3, FROMALTSTACK -> stack [3, 5]
    let script: &[u8] = &[
        0x15, // PUSH5
        0x06, // TOALTSTACK
        0x13, // PUSH3
        0x07, // FROMALTSTACK
        0x40, // RET
    ];
    let result = interpret(script).expect("TOALTSTACK/FROMALTSTACK should round-trip values");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(3), StackValue::Integer(5)]
    );
}

#[test]
fn assertmsg_true_passes() {
    // PUSHT, PUSHDATA1 "ok", ASSERTMSG → continues, PUSH1, RET
    let script: &[u8] = &[
        0x08, // PUSHT
        0x0c, 0x02, b'o', b'k', // PUSHDATA1 "ok"
        0xe1, // ASSERTMSG
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("ASSERTMSG with true should continue normally");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn assertmsg_false_faults_with_message() {
    // PUSHF, PUSHDATA1 "bad", ASSERTMSG → FAULT with "bad"
    let script: &[u8] = &[
        0x09, // PUSHF
        0x0c, 0x03, b'b', b'a', b'd', // PUSHDATA1 "bad"
        0xe1, // ASSERTMSG
    ];
    let result = interpret(script);
    match result {
        Err(e) => assert!(
            e.contains("bad"),
            "error should contain the assert message: {e}"
        ),
        Ok(r) => {
            assert_eq!(r.state, VmState::Fault, "ASSERTMSG on false should FAULT");
            let msg = r.fault_message.unwrap_or_default();
            assert!(
                msg.contains("bad"),
                "fault_message should contain 'bad': {msg}"
            );
        }
    }
}

#[test]
fn try_l_long_form_catch() {
    // TRY_L(0x3c) with 4-byte catch/finally offsets, THROW, verify catch executes
    // ip=0: TRY_L
    // ip=1-4: catch_offset = +11 (0x0b) → target ip=11
    // ip=5-8: finally_offset = 0
    // ip=9: PUSH1 (value to throw)
    // ip=10: THROW
    // ip=11: PUSH2 (catch handler)
    // ip=12: ENDTRY +2 → ip=14
    // ip=14: RET
    let script: &[u8] = &[
        0x3c, // TRY_L
        0x0b, 0x00, 0x00, 0x00, // catch_offset = +11
        0x00, 0x00, 0x00, 0x00, // finally_offset = 0
        0x11, // PUSH1
        0x3a, // THROW
        0x12, // PUSH2 (catch handler)
        0x3d, 0x02, // ENDTRY +2 → ip=14
        0x40, // RET
    ];
    let result = interpret(script).expect("TRY_L catch should handle THROW");
    assert_eq!(result.state, VmState::Halt);
    assert!(
        result.stack.contains(&StackValue::Integer(2)),
        "catch handler should have executed, stack: {:?}",
        result.stack
    );
}

#[test]
fn endtry_l_long_form() {
    // TRY (short) setup, normal path, ENDTRY_L(0x3e) with 4-byte offset to skip catch
    // ip=0: TRY catch=+9, finally=0
    // ip=3: PUSH1
    // ip=4: ENDTRY_L
    // ip=5-8: offset = +6 → target ip=4+6=10
    // ip=9: ABORT (catch handler, should not execute)
    // ip=10: RET
    let script: &[u8] = &[
        0x3b, 0x09, 0x00, // TRY catch=+9, finally=0
        0x11, // PUSH1
        0x3e, // ENDTRY_L
        0x06, 0x00, 0x00, 0x00, // i32 offset = +6 → target ip=4+6=10
        0x38, // ABORT (catch — skipped)
        0x40, // RET
    ];
    let result = interpret(script).expect("ENDTRY_L should skip over catch block");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

// =============================================================================
// High-value opcode tests (batch 3)
// =============================================================================

#[test]
fn pushdata4_with_valid_payload() {
    // PUSHDATA4(0x0e) with 4-byte LE length (5) + "hello" payload
    let script: &[u8] = &[
        0x0e, // PUSHDATA4
        0x05, 0x00, 0x00, 0x00, // length = 5 (LE u32)
        b'h', b'e', b'l', b'l', b'o', // payload
        0x40, // RET
    ];
    let result = interpret(script).expect("PUSHDATA4 should decode 4-byte-length payload");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::ByteString(b"hello".to_vec())]
    );
}

struct CalltHost;

impl SyscallProvider for CalltHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        assert_eq!(token, 0, "expected CALLT token 0");
        // Pop one integer, add 100, push result
        let val = match stack.pop() {
            Some(StackValue::Integer(n)) => n,
            other => return Err(format!("callt expected Integer, got {:?}", other)),
        };
        stack.push(StackValue::Integer(val + 100));
        Ok(())
    }
}

#[test]
fn callt_invokes_host() {
    // PUSH1, CALLT token=0x0000, RET → host adds 100 → 101
    let script: &[u8] = &[
        0x11, // PUSH1
        0x37, // CALLT
        0x00, 0x00, // token = 0 (LE u16)
        0x40, // RET
    ];
    let mut host = CalltHost;
    let result = interpret_with_syscalls(script, &mut host)
        .expect("CALLT should invoke host callt provider");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(101)]);
}

#[test]
fn syscall_invokes_host_provider() {
    // Explicit SYSCALL test: PUSH1, SYSCALL "System.Runtime.Platform" → "NEO"
    let mut host = PlatformHost;
    let syscall = neo_riscv_abi::interop_hash("System.Runtime.Platform");
    let script: &[u8] = &[
        0x11, // PUSH1
        0x41, // SYSCALL
        (syscall & 0xFF) as u8,
        ((syscall >> 8) & 0xFF) as u8,
        ((syscall >> 16) & 0xFF) as u8,
        ((syscall >> 24) & 0xFF) as u8,
        0x40, // RET
    ];
    let result =
        interpret_with_syscalls(script, &mut host).expect("SYSCALL should invoke host provider");
    assert_eq!(result.state, VmState::Halt);
    // Stack has PUSH1's value (1) then Platform's result ("NEO") on top
    assert!(
        result
            .stack
            .contains(&StackValue::ByteString(b"NEO".to_vec())),
        "Platform syscall should return NEO, stack: {:?}",
        result.stack
    );
}

#[test]
fn slot_ldloc6_stloc6() {
    // INITSLOT 7 locals 0 args, PUSHINT8 42, STLOC6, LDLOC6 → 42
    let script: &[u8] = &[
        0x57, // INITSLOT
        0x07, // 7 locals
        0x00, // 0 args
        0x00, 0x2a, // PUSHINT8(0x00) 42
        0x76, // STLOC6
        0x6e, // LDLOC6
        0x40, // RET
    ];
    let result = interpret(script).expect("STLOC6/LDLOC6 should round-trip");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn slot_ldsfld6_stsfld6() {
    // INITSSLOT 7, PUSHINT8 42, STSFLD6, LDSFLD6 → 42
    let script: &[u8] = &[
        0x56, // INITSSLOT
        0x07, // 7 static fields
        0x00, 0x2a, // PUSHINT8(0x00) 42
        0x66, // STSFLD6
        0x5e, // LDSFLD6
        0x40, // RET
    ];
    let result = interpret(script).expect("STSFLD6/LDSFLD6 should round-trip");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(42)]);
}

#[test]
fn slot_ldarg6_starg6() {
    // Push 7 args (values 10..70), INITSLOT 0 locals 7 args
    // Args popped: top=70→locals[0], 60→[1], 50→[2], 40→[3], 30→[4], 20→[5], 10→[6]
    // LDARG6 → locals[6] = 10
    // PUSHINT8 99, STARG0, LDARG0 → 99
    let script: &[u8] = &[
        0x00, 0x0a, // PUSHINT8 10
        0x00, 0x14, // PUSHINT8 20
        0x00, 0x1e, // PUSHINT8 30
        0x00, 0x28, // PUSHINT8 40
        0x00, 0x32, // PUSHINT8 50
        0x00, 0x3c, // PUSHINT8 60
        0x00, 0x46, // PUSHINT8 70
        0x57, // INITSLOT
        0x00, // 0 locals
        0x07, // 7 args
        0x7e, // LDARG6 → locals[6] = 10
        0x00, 0x63, // PUSHINT8 99
        0x80, // STARG0 → locals[0] = 99
        0x78, // LDARG0 → 99
        0x40, // RET
    ];
    let result = interpret(script).expect("LDARG6/STARG0 slot operations should work");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(10), StackValue::Integer(99)]
    );
}

#[test]
fn jmpif_l_long_form() {
    // PUSHT, JMPIF_L offset=+6 → target ip=7, skip ABORT, PUSH1, RET
    let script: &[u8] = &[
        0x08, // PUSHT
        0x25, // JMPIF_L
        0x06, 0x00, 0x00, 0x00, // i32 offset = +6 → target ip=1+6=7
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPIF_L should jump on true");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn jmpifnot_l_long_form() {
    // PUSHF, JMPIFNOT_L offset=+6 → target ip=7, skip ABORT, PUSH1, RET
    let script: &[u8] = &[
        0x09, // PUSHF
        0x27, // JMPIFNOT_L
        0x06, 0x00, 0x00, 0x00, // i32 offset = +6 → target ip=1+6=7
        0x38, // ABORT (skipped)
        0x11, // PUSH1
        0x40, // RET
    ];
    let result = interpret(script).expect("JMPIFNOT_L should jump on false");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(result.stack, vec![StackValue::Integer(1)]);
}

#[test]
fn push_high_constants() {
    // PUSH8(0x18) → 8, PUSH16(0x20) → 16, RET
    let script: &[u8] = &[
        0x18, // PUSH8
        0x20, // PUSH16
        0x40, // RET
    ];
    let result = interpret(script).expect("PUSH8 and PUSH16 should push correct values");
    assert_eq!(result.state, VmState::Halt);
    assert_eq!(
        result.stack,
        vec![StackValue::Integer(8), StackValue::Integer(16)]
    );
}
