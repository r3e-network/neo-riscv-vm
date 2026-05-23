pub(crate) struct FfiInitializerStoragePutState {
    pub(crate) init_complete_count: usize,
    pub(crate) observed_keys: Vec<Vec<u8>>,
}
