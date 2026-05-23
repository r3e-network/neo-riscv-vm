use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct BlockLikeHost {
    pub(crate) block_like: StackValue,
}

impl SyscallProvider for BlockLikeHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if token != 2 {
            return Err(format!("unexpected callt token {token}"));
        }
        stack.clear();
        stack.push(self.block_like.clone());
        Ok(())
    }
}
