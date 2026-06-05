use super::{ARENA_SIZE, raw_buffer::RawBuffer};

#[repr(align(64))]
pub(crate) struct AlignedArena(RawBuffer<ARENA_SIZE>);

unsafe impl Sync for AlignedArena {}

impl AlignedArena {
    pub(crate) const fn new() -> Self {
        Self(RawBuffer::new())
    }

    pub(crate) unsafe fn as_mut_ptr(&self) -> *mut u8 {
        // SAFETY: caller upholds RawBuffer::as_mut_ptr's contract (no aliasing
        // mutable references to the arena are live).
        unsafe { self.0.as_mut_ptr() }
    }
}
