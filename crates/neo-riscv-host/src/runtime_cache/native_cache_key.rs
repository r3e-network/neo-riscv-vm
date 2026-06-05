#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct NativeCacheKey {
    pub(super) binary_hash: [u8; 32],
    pub(super) aux_size: u32,
}
