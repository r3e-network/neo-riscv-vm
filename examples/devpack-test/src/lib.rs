#![no_std]
#![no_main]

extern crate alloc;

use neo_riscv_devpack::storage;

#[global_allocator]
static ALLOCATOR: polkavm_derive::allocator::BumpAllocator = polkavm_derive::allocator::BumpAllocator;

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

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
