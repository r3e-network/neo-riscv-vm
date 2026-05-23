use neo_riscv_abi::StackValue;

pub(crate) struct FfiOracleSuccessState {
    pub(crate) stored: Option<Vec<StackValue>>,
}
