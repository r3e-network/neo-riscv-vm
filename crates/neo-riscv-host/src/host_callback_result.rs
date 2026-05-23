#[derive(Debug)]
pub struct HostCallbackResult {
    pub stack: Vec<neo_riscv_abi::StackValue>,
}
