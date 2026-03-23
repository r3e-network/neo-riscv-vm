#![no_std]

extern crate alloc;

pub mod api_ids;
pub mod codec;
pub mod events;
pub mod ffi;
pub mod native;
pub mod parser;
pub mod signing;
pub mod storage;
pub mod syscalls;
pub mod types;

// Re-export commonly used types
pub use storage::{delete, get, put};
pub use syscalls::{contract_call, contract_create, contract_update};
pub mod runtime {
    pub use crate::syscalls::{runtime_check_witness, runtime_log, runtime_notify};
}
