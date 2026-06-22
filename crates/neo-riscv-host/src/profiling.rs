//! Process-memory profiling counters for the host runtime.
//!
//! **Limitation:** these counters are currently *read-only stubs* — nothing in the
//! crate writes to `PEAK_MEMORY`/`CURRENT_MEMORY` (no sampler is wired up), so
//! [`get_peak_memory`] and [`get_current_memory`] always return `0`. The functions
//! are retained for API stability (they are re-exported from the crate root) and
//! as hooks for a future periodic sampler (e.g. a background thread calling the
//! platform `getrusage` equivalent). Do not rely on non-zero values until a
//! sampler is implemented. The C# adapter uses its own per-syscall profiling
//! (`NativeRiscvVmBridge.Profiling.cs`) and does not depend on these counters.

use std::sync::atomic::{AtomicUsize, Ordering};

static PEAK_MEMORY: AtomicUsize = AtomicUsize::new(0);
static CURRENT_MEMORY: AtomicUsize = AtomicUsize::new(0);

/// Return the peak process memory (in bytes) observed since the last
/// [`reset`].
pub fn get_peak_memory() -> usize {
    PEAK_MEMORY.load(Ordering::Relaxed)
}

/// Return the most recently sampled process memory (in bytes).
pub fn get_current_memory() -> usize {
    CURRENT_MEMORY.load(Ordering::Relaxed)
}

/// Reset both the peak and current memory counters to zero.
pub fn reset() {
    PEAK_MEMORY.store(0, Ordering::Relaxed);
    CURRENT_MEMORY.store(0, Ordering::Relaxed);
}
