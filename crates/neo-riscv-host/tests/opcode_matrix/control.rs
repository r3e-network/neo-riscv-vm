// Control flow opcode tests

#[cfg(test)]
mod tests {
    use neo_riscv_abi::StackValue;
    use neo_riscv_host::execute_script;

    #[test]
    fn jmpif_true_skips() {
        // PUSHT, JMPIF +3, PUSH1, PUSH2
        let script = vec![0x08, 0x24, 0x03, 0x11, 0x12];
        let result = execute_script(&script).unwrap();
        // True condition jumps, skips PUSH1, only PUSH2 executes
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(2));
    }

    #[test]
    fn jmpif_false_continues() {
        // PUSHF, JMPIF +3, PUSH1, PUSH2
        let script = vec![0x09, 0x24, 0x03, 0x11, 0x12];
        let result = execute_script(&script).unwrap();
        // False condition doesn't jump, both execute
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack[0], StackValue::Integer(1));
        assert_eq!(result.stack[1], StackValue::Integer(2));
    }

    #[test]
    fn jmpifnot_true_continues() {
        // PUSHT, JMPIFNOT +3, PUSH1, PUSH2
        let script = vec![0x08, 0x26, 0x03, 0x11, 0x12];
        let result = execute_script(&script).unwrap();
        // True condition doesn't jump (JMPIFNOT), both execute
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack[0], StackValue::Integer(1));
        assert_eq!(result.stack[1], StackValue::Integer(2));
    }

    #[test]
    fn ret_exits() {
        // PUSH1, RET, PUSH2
        let script = vec![0x11, 0x40, 0x12];
        let result = execute_script(&script).unwrap();
        // RET exits, PUSH2 never executes
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(1));
    }
}
