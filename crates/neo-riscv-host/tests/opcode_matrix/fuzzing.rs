#[cfg(test)]
mod tests {
    use neo_riscv_abi::StackValue;
    use neo_riscv_host::execute_script;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 16,
            max_shrink_iters: 0,
            .. ProptestConfig::default()
        })]

        #[test]
        fn pushint8_round_trips(value in any::<i8>()) {
            let script = vec![0x00, value as u8, 0x40]; // PUSHINT8, RET
            let result = execute_script(&script).expect("PUSHINT8 script should execute");
            prop_assert_eq!(result.stack, vec![StackValue::Integer(i64::from(value))]);
        }

        #[test]
        fn pushdata1_round_trips(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
            let mut script = vec![0x0c, bytes.len() as u8]; // PUSHDATA1
            script.extend_from_slice(&bytes);
            script.push(0x40); // RET

            let result = execute_script(&script).expect("PUSHDATA1 script should execute");
            prop_assert_eq!(result.stack, vec![StackValue::ByteString(bytes)]);
        }
    }
}
