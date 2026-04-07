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
pub use codec::{
    decode_bool_result, decode_bytes_result, decode_int_result, decode_string_result,
    encode_bytes, encode_int_params, encode_string_params,
};
pub use events::{
    notify, notify_args, notify_int, notify_key_value, notify_string, notify_two_strings,
};
pub use parser::{format_stack_value, parse_int_result, parse_stack_value, parse_string_result};
pub use signing::{check_witness, verify_multisig, verify_signature};
pub use storage::{delete, get, put};
pub use syscalls::{contract_call, contract_create, contract_update};
pub mod runtime {
    pub use crate::syscalls::{
        runtime_burn_gas, runtime_check_witness, runtime_current_signers,
        runtime_gas_left, runtime_get_address_version, runtime_get_calling_script_hash,
        runtime_get_entry_script_hash, runtime_get_executing_script_hash,
        runtime_get_invocation_counter, runtime_get_network, runtime_get_notifications,
        runtime_get_random, runtime_get_script_container, runtime_get_time,
        runtime_load_script, runtime_log, runtime_notify, runtime_platform,
    };
}
