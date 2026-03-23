// Type conversion and MaxItemSize tests

#[cfg(test)]
mod tests {
    use neo_riscv_abi::StackValue;
    use neo_riscv_host::execute_script;

    #[allow(dead_code)]
    const MAX_ITEM_SIZE: usize = 1024 * 1024; // 1MB

    #[test]
    fn convert_int_to_bool_true() {
        // PUSH1, CONVERT to Boolean (0x20)
        let script = vec![0x11, 0xdb, 0x20];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Boolean(true));
    }

    #[test]
    fn convert_int_to_bool_false() {
        // PUSH0, CONVERT to Boolean (0x20)
        let script = vec![0x10, 0xdb, 0x20];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Boolean(false));
    }

    #[test]
    fn istype_integer() {
        // PUSH1, ISTYPE Integer (0x21)
        let script = vec![0x11, 0xd9, 0x21];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Boolean(true));
    }

    #[test]
    fn istype_boolean_false() {
        // PUSH1, ISTYPE Boolean (0x20)
        let script = vec![0x11, 0xd9, 0x20];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Boolean(false));
    }

    #[test]
    fn newbuffer_at_limit() {
        // PUSHINT32 1048576, NEWBUFFER, RET
        let script = vec![0x02, 0x00, 0x00, 0x10, 0x00, 0x88, 0x40];
        let result = execute_script(&script).expect("NEWBUFFER at MaxItemSize should succeed");
        assert_eq!(result.stack.len(), 1);
        match &result.stack[0] {
            StackValue::ByteString(bytes) => assert_eq!(bytes.len(), MAX_ITEM_SIZE),
            other => panic!("expected ByteString buffer, got {other:?}"),
        }
    }

    #[test]
    fn newbuffer_exceeds_limit() {
        // PUSHINT32 1048577, NEWBUFFER
        let script = vec![0x02, 0x01, 0x00, 0x10, 0x00, 0x88];
        let result = execute_script(&script).expect_err("NEWBUFFER > MaxItemSize should fail");
        assert!(
            result.contains("MaxItemSize") || result.contains("size") || result.contains("FAULT"),
            "expected size-related fault, got: {result}",
        );
    }

    #[test]
    fn cat_exceeds_limit() {
        let chunk_len = (MAX_ITEM_SIZE / 2) + 1;
        let mut script = Vec::with_capacity((chunk_len * 2) + 16);
        for _ in 0..2 {
            script.push(0x0e); // PUSHDATA4
            script.extend_from_slice(&(chunk_len as u32).to_le_bytes());
            script.extend(std::iter::repeat_n(0xaa, chunk_len));
        }
        script.push(0x8b); // CAT
        script.push(0x40); // RET

        let result = execute_script(&script).expect_err("CAT above MaxItemSize should fail");
        assert!(
            result.contains("MaxItemSize") || result.contains("size") || result.contains("FAULT"),
            "expected size-related CAT fault, got: {result}",
        );
    }
}
