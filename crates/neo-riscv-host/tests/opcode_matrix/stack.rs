// Stack underflow edge case tests

#[cfg(test)]
mod tests {
    use neo_riscv_abi::StackValue;
    use neo_riscv_host::execute_script;

    #[test]
    fn drop_empty_stack_faults() {
        let script = vec![0x45]; // DROP opcode
        let result = execute_script(&script);
        assert!(result.is_err(), "DROP on empty stack should FAULT");
    }

    #[test]
    fn dup_empty_stack_faults() {
        let script = vec![0x4a]; // DUP opcode
        let result = execute_script(&script);
        assert!(result.is_err(), "DUP on empty stack should FAULT");
    }

    #[test]
    fn dup_basic() {
        // PUSH5, DUP
        let script = vec![0x15, 0x4a];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack[0], StackValue::Integer(5));
        assert_eq!(result.stack[1], StackValue::Integer(5));
    }

    #[test]
    fn swap_basic() {
        // PUSH1, PUSH2, SWAP
        let script = vec![0x11, 0x12, 0x50];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack[0], StackValue::Integer(2));
        assert_eq!(result.stack[1], StackValue::Integer(1));
    }

    #[test]
    fn rot_basic() {
        // PUSH1, PUSH2, PUSH3, ROT
        let script = vec![0x11, 0x12, 0x13, 0x51];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 3);
        // ROT: bottom moves to top [1,2,3] -> [2,3,1]
        assert_eq!(result.stack[0], StackValue::Integer(2));
        assert_eq!(result.stack[1], StackValue::Integer(3));
        assert_eq!(result.stack[2], StackValue::Integer(1));
    }

    #[test]
    fn reverse3_basic() {
        // PUSH1, PUSH2, PUSH3, REVERSE3
        let script = vec![0x11, 0x12, 0x13, 0x53];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 3);
        assert_eq!(result.stack[0], StackValue::Integer(3));
        assert_eq!(result.stack[1], StackValue::Integer(2));
        assert_eq!(result.stack[2], StackValue::Integer(1));
    }
}
