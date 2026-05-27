use std::sync::atomic::{AtomicUsize, Ordering};

static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);
static CURRENT_MEMORY: AtomicUsize = AtomicUsize::new(0);

pub fn get_peak_memory() -> usize {
    PEAK_MEMORY.load(Ordering::Relaxed)
}

pub fn get_current_memory() -> usize {
    CURRENT_MEMORY.load(Ordering::Relaxed)
}

pub fn reset() {
    PEAK_MEMORY.store(0, Ordering::Relaxed);
    CURRENT_MEMORY.store(0, Ordering::Relaxed);
}
