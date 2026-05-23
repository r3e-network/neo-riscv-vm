use core::cell::UnsafeCell;

pub(crate) struct RawBuffer<const N: usize>(UnsafeCell<[u8; N]>);

unsafe impl<const N: usize> Sync for RawBuffer<N> {}

impl<const N: usize> RawBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self(UnsafeCell::new([0; N]))
    }

    pub(crate) unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.0.get().cast::<u8>()
    }
}
