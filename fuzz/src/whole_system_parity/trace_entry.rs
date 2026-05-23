use neo_riscv_abi::StackValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TraceEntry {
    pub(crate) api: u32,
    pub(crate) ip: usize,
    pub(crate) stack: Vec<StackValue>,
}
