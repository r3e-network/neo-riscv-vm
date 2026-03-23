use std::sync::atomic::{AtomicUsize, Ordering};

static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);
static CURRENT_MEMORY: AtomicUsize = AtomicUsize::new(0);

#[allow(dead_code)]
pub fn record_allocation(size: usize) {
    let current = CURRENT_MEMORY.fetch_add(size, Ordering::Relaxed) + size;
    PEAK_MEMORY.fetch_max(current, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn record_deallocation(size: usize) {
    CURRENT_MEMORY.fetch_sub(size, Ordering::Relaxed);
}

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
