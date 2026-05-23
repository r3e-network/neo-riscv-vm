use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct CalltHost;

impl SyscallProvider for CalltHost {
    fn syscall(
        &mut self,
        api: u32,
        _ip: usize,
        _stack: &mut Vec<StackValue>,
    ) -> Result<(), String> {
        Err(format!("unexpected syscall 0x{api:08x}"))
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        assert_eq!(token, 0, "expected CALLT token 0");
        let val = match stack.pop() {
            Some(StackValue::Integer(n)) => n,
            other => return Err(format!("callt expected Integer, got {:?}", other)),
        };
        stack.push(StackValue::Integer(val + 100));
        Ok(())
    }
}
