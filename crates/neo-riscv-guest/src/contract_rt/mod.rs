//! Contract runtime helpers for C#-compiled PolkaVM smart contracts.
//!
//! Common VM state and opcode semantics live in `neo-vm-rs`. This module keeps
//! only the RISC-V guest boundary: ABI entry setup, syscall bridging, and small
//! target-specific runtime glue.

use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

pub mod stack_value;

#[cfg(target_arch = "riscv32")]
#[path = "../../../neo-riscv-guest-module/src/mem_intrinsics.rs"]
mod mem_intrinsics;

pub use stack_value::StackValue;

use neo_riscv_abi::{ExecutionResult, VmContext, runtime::RuntimeStack};

/// Signature of a syscall bridge function.
///
/// The bridge is responsible for marshaling stack arguments to the host,
/// invoking the requested interop API, and pushing callback results.
pub type SyscallBridgeFn = fn(&mut Context, u32);

/// Global syscall bridge function pointer.
///
/// SAFETY: Written once at startup before any syscall fires. The PolkaVM guest
/// is single-threaded, so there is no data race.
static mut SYSCALL_BRIDGE: Option<SyscallBridgeFn> = None;

/// Register a syscall bridge that `Context::syscall()` delegates to.
pub fn set_syscall_bridge(function: SyscallBridgeFn) {
    unsafe {
        SYSCALL_BRIDGE = Some(function);
    }
}

/// RISC-V contract execution context.
///
/// Common VM state is stored in [`VmContext`]. This wrapper keeps only the
/// RISC-V-specific syscall surface so opcode semantics can be called directly
/// through `neo_riscv_abi::runtime::ops`.
pub struct Context {
    vm: VmContext,
}

impl Context {
    /// Creates a new context from ABI stack values supplied by the host.
    #[must_use]
    pub fn from_abi_stack(stack: Vec<StackValue>) -> Self {
        Self {
            vm: VmContext::from_stack(stack),
        }
    }

    /// Consumes this wrapper and returns the shared VM context.
    #[must_use]
    pub fn into_vm_context(self) -> VmContext {
        self.vm
    }

    /// Converts the current VM state into an ABI execution result.
    #[must_use]
    pub fn to_execution_result(self, fee_consumed_pico: i64) -> ExecutionResult {
        self.vm.into_execution_result(fee_consumed_pico)
    }

    /// Invokes a host syscall by interop hash.
    pub fn syscall(&mut self, hash: u32) {
        let bridge = unsafe { SYSCALL_BRIDGE };
        if let Some(function) = bridge {
            function(self, hash);
        }
    }

    /// Stub: call a token-referenced method.
    ///
    /// Cross-contract invocation is provided by the host bridge, not by the
    /// guest runtime itself.
    pub fn call_token(&mut self, _token: u16) {
        self.vm
            .fault("CALLT: not yet implemented (requires host bridge)");
    }

    /// Stub: call the address on top of the stack.
    pub fn calla(&mut self) {
        self.vm
            .fault("CALLA: not yet implemented (requires host bridge)");
    }
}

impl Deref for Context {
    type Target = VmContext;

    fn deref(&self) -> &Self::Target {
        &self.vm
    }
}

impl DerefMut for Context {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vm
    }
}

impl RuntimeStack for Context {
    fn pop_value(&mut self) -> StackValue {
        self.vm.pop_value()
    }

    fn push_value(&mut self, value: StackValue) {
        self.vm.push_value(value);
    }

    fn top_value_mut(&mut self) -> Option<&mut StackValue> {
        self.vm.top_value_mut()
    }

    fn stack_values(&self) -> &[StackValue] {
        self.vm.stack_values()
    }

    fn stack_values_mut(&mut self) -> &mut Vec<StackValue> {
        self.vm.stack_values_mut()
    }

    fn fault(&mut self, message: &str) {
        self.vm.fault(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use neo_riscv_abi::{VmState, runtime::ops};

    #[test]
    fn from_abi_stack_roundtrip() {
        let abi_stack = vec![
            StackValue::Integer(42),
            StackValue::Boolean(true),
            StackValue::Null,
        ];
        let context = Context::from_abi_stack(abi_stack);
        let result = context.to_execution_result(1000);

        assert_eq!(result.state, VmState::Halt);
        assert_eq!(result.fee_consumed_pico, 1000);
        assert_eq!(result.stack.len(), 3);
        assert_eq!(result.stack[0], StackValue::Integer(42));
    }

    #[test]
    fn shared_runtime_ops_execute_against_riscv_context() {
        let mut context = Context::from_abi_stack(vec![]);

        context.push_int(10);
        context.push_int(3);
        ops::arithmetic::sub(&mut context);

        assert_eq!(context.pop(), StackValue::Integer(7));
    }

    #[test]
    fn riscv_syscall_stubs_fault_through_shared_vm_context() {
        let mut context = Context::from_abi_stack(vec![]);

        context.call_token(0);

        assert_eq!(context.state, VmState::Fault);
        assert_eq!(
            context.fault_message.as_deref(),
            Some("CALLT: not yet implemented (requires host bridge)")
        );
    }
}
