use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use super::{arena_ptr, runtime_state, ALLOC_ARENA_SIZE, ALLOC_BASE_OFFSET};

pub(crate) struct ResettableBumpAllocator;

unsafe impl Sync for ResettableBumpAllocator {}

impl ResettableBumpAllocator {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) unsafe fn reset(&self) {
        let state = &mut runtime_state().bump;
        state.clear();
    }

    unsafe fn alloc_from_arena(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return NonNull::<u8>::dangling().as_ptr();
        }

        let state = &mut runtime_state().bump;
        let base = arena_ptr(ALLOC_BASE_OFFSET) as usize;
        let align_mask = layout.align() - 1;
        // Use checked arithmetic to prevent 32-bit integer overflow when
        // base + offset + align_mask wraps around on riscv32.
        let Some(current) = base.checked_add(state.offset) else {
            state.fail_count = state.fail_count.saturating_add(1);
            state.fail_size = layout.size();
            state.fail_align = layout.align();
            return core::ptr::null_mut();
        };
        let Some(sum) = current.checked_add(align_mask) else {
            state.fail_count = state.fail_count.saturating_add(1);
            state.fail_size = layout.size();
            state.fail_align = layout.align();
            return core::ptr::null_mut();
        };
        let aligned = sum & !align_mask;
        let Some(end) = aligned.checked_add(layout.size()) else {
            state.fail_count = state.fail_count.saturating_add(1);
            state.fail_size = layout.size();
            state.fail_align = layout.align();
            return core::ptr::null_mut();
        };
        if end > base + ALLOC_ARENA_SIZE {
            state.fail_count = state.fail_count.saturating_add(1);
            state.fail_size = layout.size();
            state.fail_align = layout.align();
            return core::ptr::null_mut();
        }

        state.offset = end - base;
        state.peak = core::cmp::max(state.peak, state.offset);
        aligned as *mut u8
    }

    pub(crate) unsafe fn peak_bytes(&self) -> u32 {
        runtime_state().bump.peak.min(u32::MAX as usize) as u32
    }

    pub(crate) unsafe fn fail_count(&self) -> u32 {
        runtime_state().bump.fail_count
    }

    pub(crate) unsafe fn fail_size(&self) -> u32 {
        runtime_state().bump.fail_size.min(u32::MAX as usize) as u32
    }

    pub(crate) unsafe fn fail_align(&self) -> u32 {
        runtime_state().bump.fail_align.min(u32::MAX as usize) as u32
    }
}

unsafe impl GlobalAlloc for ResettableBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_from_arena(layout)
    }

    // SAFETY: dealloc is intentionally a no-op. This allocator uses a bump-pointer
    // strategy where all allocations live in a single 128 MB arena. Individual
    // deallocations are not possible; instead, the entire arena is bulk-reset via
    // `ALLOCATOR.reset()` between guest executions. This means:
    //
    // 1. Drop is NEVER called for heap-allocated types in the guest. Guest code
    //    MUST NOT rely on Drop for resource cleanup (file handles, locks, etc.);
    //    these resource types are not available in the RISC-V guest environment.
    //
    // 2. Memory usage grows monotonically during a single execution. Operators
    //    must ensure the 128 MB arena is sufficient for the worst-case script.
    //
    // This is the standard pattern for single-use bump allocators in embedded
    // and guest VM contexts.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = (ptr, layout);
    }
}
