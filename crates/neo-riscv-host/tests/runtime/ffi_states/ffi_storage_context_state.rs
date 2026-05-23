use neo_riscv_abi::StackValue;

pub(crate) struct FfiStorageContextState {
    pub(crate) calls: Vec<(u32, Vec<StackValue>)>,
    pub(crate) token: Vec<u8>,
}
