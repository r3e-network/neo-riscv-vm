#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct NativeCacheKey {
    pub(super) binary_hash: u64,
    pub(super) aux_size: u32,
}
