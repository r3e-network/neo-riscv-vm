use neo_riscv_abi::StackValue;

pub(crate) struct FfiInitializerWitnessState {
    pub(crate) init_complete_count: usize,
    pub(crate) observed_checkwitness: Option<Vec<StackValue>>,
}
