use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

use super::{DeterministicModel, Scenario};

pub(crate) struct GuestParityProvider {
    pub(crate) model: DeterministicModel,
}

impl GuestParityProvider {
    pub(crate) fn new(scenario: Scenario) -> Self {
        Self {
            model: DeterministicModel::new(scenario),
        }
    }
}

impl SyscallProvider for GuestParityProvider {
    fn syscall(&mut self, api: u32, ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let next_stack = self.model.handle(api, ip, stack.as_slice())?;
        *stack = next_stack;
        Ok(())
    }
}
