use super::{raw_buffer::RawBuffer, ARENA_SIZE};

#[repr(align(64))]
pub(crate) struct AlignedArena(RawBuffer<ARENA_SIZE>);

unsafe impl Sync for AlignedArena {}

impl AlignedArena {
    pub(crate) const fn new() -> Self {
        Self(RawBuffer::new())
    }

    pub(crate) unsafe fn as_mut_ptr(&self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}
