use neo_riscv_abi::StackValue;

pub(crate) struct FfiAttributeState {
    pub(crate) observed_checkwitness: Option<Vec<StackValue>>,
}
