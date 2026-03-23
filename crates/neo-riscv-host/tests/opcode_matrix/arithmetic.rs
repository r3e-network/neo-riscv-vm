// Integer overflow edge case tests

#[cfg(test)]
mod tests {
    use neo_riscv_abi::StackValue;
    use neo_riscv_host::execute_script;

    #[test]
    fn add_basic() {
        // PUSH1, PUSH2, ADD
        let script = vec![0x11, 0x12, 0x9e];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(3));
    }

    #[test]
    fn sub_basic() {
        // PUSH5, PUSH2, SUB
        let script = vec![0x15, 0x12, 0x9f];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(3));
    }

    #[test]
    fn mul_basic() {
        // PUSH4, PUSH3, MUL
        let script = vec![0x14, 0x13, 0xa0];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(12));
    }

    #[test]
    fn div_basic() {
        // PUSH10, PUSH2, DIV
        let script = vec![0x1a, 0x12, 0xa1];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(5));
    }

    #[test]
    fn div_by_zero_faults() {
        // PUSH5, PUSH0, DIV
        let script = vec![0x15, 0x10, 0xa1];
        let result = execute_script(&script);
        assert!(result.is_err(), "DIV by zero should FAULT");
    }

    #[test]
    fn mod_basic() {
        // PUSH10, PUSH3, MOD
        let script = vec![0x1a, 0x13, 0xa2];
        let result = execute_script(&script).unwrap();
        assert_eq!(result.stack.len(), 1);
        assert_eq!(result.stack[0], StackValue::Integer(1));
    }
}
