#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use neo_riscv_devpack::storage;

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

#[no_mangle]
pub extern "C" fn invoke(_method: *const u8, _args: *const u8) -> i32 {
    let key = b"counter";
    let value = storage::get(key);

    let count = match value {
        Some(v) if v.len() >= 4 => i32::from_le_bytes([v[0], v[1], v[2], v[3]]),
        _ => 0,
    };

    let new_count = count + 1;
    storage::put(key, &new_count.to_le_bytes());

    new_count
}

#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
