use core::cell::UnsafeCell;

use super::runtime_state::RuntimeState;

pub(crate) struct RuntimeStateCell(UnsafeCell<RuntimeState>);

unsafe impl Sync for RuntimeStateCell {}

impl RuntimeStateCell {
    pub(crate) const fn new() -> Self {
        Self(UnsafeCell::new(RuntimeState::new()))
    }

    pub(crate) unsafe fn get_mut(&self) -> &'static mut RuntimeState {
        // SAFETY: caller guarantees no aliasing &mut to the cell's contents is
        // live (single-threaded guest; see this type's Sync impl).
        unsafe { &mut *self.0.get() }
    }
}
