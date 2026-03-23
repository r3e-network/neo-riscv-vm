#[cfg(test)]
mod tests {
    use neo_riscv_host::execute_contract;

    #[test]
    fn test_counter_increment() {
        let binary = include_bytes!("../target/counter.polkavm");
        let result = execute_contract(binary, "increment", &[]).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_counter_get() {
        let binary = include_bytes!("../target/counter.polkavm");
        let result = execute_contract(binary, "get", &[]).unwrap();
        assert_eq!(result, 0);
    }
}
