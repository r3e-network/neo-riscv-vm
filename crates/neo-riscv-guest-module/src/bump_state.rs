#[derive(Default)]
pub(crate) struct BumpState {
    pub(crate) offset: usize,
    pub(crate) peak: usize,
    pub(crate) fail_count: u32,
    pub(crate) fail_size: usize,
    pub(crate) fail_align: usize,
}

impl BumpState {
    pub(crate) const fn new() -> Self {
        Self {
            offset: 0,
            peak: 0,
            fail_count: 0,
            fail_size: 0,
            fail_align: 0,
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::new();
    }
}
