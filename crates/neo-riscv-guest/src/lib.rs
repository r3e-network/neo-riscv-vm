//! Neo RISC-V guest VM facade.
//!
//! The canonical NeoVM2 interpreter now lives in `neo-vm-rs`. This crate keeps
//! the historical `neo_riscv_guest` API surface for PolkaVM guest modules,
//! host tests, fuzzers, and downstream callers while avoiding a second private
//! interpreter implementation.

#![no_std]

extern crate alloc;

pub mod contract_rt;

pub use neo_riscv_abi::{runtime, semantics};
pub use neo_vm_rs::{
    BackendKind, CALLT_MARKER, CALLT_MARKER_HI, ExecutionResult, INITIALIZER_COMPLETE_MARKER,
    StackValue, SyscallProvider, VmState, fast_codec, interpret, interpret_with_stack_and_syscalls,
    interpret_with_stack_and_syscalls_at, interpret_with_stack_and_syscalls_at_with_initializer,
    interpret_with_stack_and_syscalls_at_with_initializer_and_result_limit,
    interpret_with_stack_and_syscalls_at_with_result_limit, interpret_with_syscalls,
    last_interpreter_ip, last_result_limit, last_result_stack_len, last_result_stage,
};
