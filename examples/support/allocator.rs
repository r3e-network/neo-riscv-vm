#![cfg(target_arch = "riscv32")]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

#[global_allocator]
static ALLOCATOR: ExampleBumpAllocator = ExampleBumpAllocator::new();

const ARENA_SIZE: usize = 64 * 1024;

struct ExampleBumpAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    offset: UnsafeCell<usize>,
}

unsafe impl Sync for ExampleBumpAllocator {}

impl ExampleBumpAllocator {
    const fn new() -> Self {
        Self {
            arena: UnsafeCell::new([0; ARENA_SIZE]),
            offset: UnsafeCell::new(0),
        }
    }
}

unsafe impl GlobalAlloc for ExampleBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let arena = &mut *self.arena.get();
        let offset = &mut *self.offset.get();

        let align_mask = layout.align() - 1;
        let start = (*offset + align_mask) & !align_mask;
        let end = match start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if end > arena.len() {
            return null_mut();
        }

        *offset = end;
        arena.as_mut_ptr().add(start)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
