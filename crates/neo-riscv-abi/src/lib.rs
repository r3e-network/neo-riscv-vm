//! Neo RISC-V ABI types and utilities.
//!
//! This crate defines the Application Binary Interface (ABI) between the RISC-V guest VM
//! and the host runtime. It provides serializable types for VM execution results, stack values,
//! and host callback communication.

#![no_std]

extern crate alloc;

pub mod callback_codec;
pub mod fast_codec;
pub mod result_codec;

pub use neo_vm_rs::{
    interop_hash, syscall_arg_count, BackendKind, ExecutionResult, StackValue, VmState,
};

#[cfg(test)]
mod tests {
    use super::{interop_hash, syscall_arg_count};

    #[test]
    fn variable_stack_syscalls_keep_the_full_stack() {
        assert_eq!(
            syscall_arg_count(interop_hash("System.Contract.CallNative")),
            usize::MAX,
        );
        assert_eq!(
            syscall_arg_count(interop_hash("System.Crypto.CheckMultisig")),
            usize::MAX,
        );
    }

    #[test]
    fn create_multisig_account_uses_neovm_descriptor_arguments() {
        assert_eq!(
            syscall_arg_count(interop_hash("System.Contract.CreateMultisigAccount")),
            2,
        );
    }
}
