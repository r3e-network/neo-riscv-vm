use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct NoopHost;

impl SyscallProvider for NoopHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}
