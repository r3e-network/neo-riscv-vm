use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct CalltNullHost {
    pub(crate) runtime_log: u32,
}

impl SyscallProvider for CalltNullHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if api == self.runtime_log {
            stack.clear();
            return Ok(());
        }
        Err(format!("unexpected syscall 0x{api:08x}"))
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if token == 2 {
            stack.clear();
            stack.push(StackValue::Null);
            return Ok(());
        }
        Err(format!("unexpected callt token {token}"))
    }
}
