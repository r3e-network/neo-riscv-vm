use core::cell::UnsafeCell;

use super::runtime_state::RuntimeState;

pub(crate) struct RuntimeStateCell(UnsafeCell<RuntimeState>);

unsafe impl Sync for RuntimeStateCell {}

impl RuntimeStateCell {
    pub(crate) const fn new() -> Self {
        Self(UnsafeCell::new(RuntimeState::new()))
    }

    pub(crate) unsafe fn get_mut(&self) -> &'static mut RuntimeState {
        &mut *self.0.get()
    }
}
