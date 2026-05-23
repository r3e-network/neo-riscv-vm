use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct InitCompleteHost {
    pub(crate) completions: usize,
}

impl SyscallProvider for InitCompleteHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }

    fn initializer_complete(&mut self, _ip: usize) -> Result<(), String> {
        self.completions += 1;
        Ok(())
    }
}
