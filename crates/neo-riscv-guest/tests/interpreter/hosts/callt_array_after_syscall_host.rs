use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct CalltArrayAfterSyscallHost;

impl SyscallProvider for CalltArrayAfterSyscallHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
        if api != platform_api {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }
        stack.clear();
        stack.push(StackValue::ByteString(b"NEO".to_vec()));
        Ok(())
    }

    fn callt(&mut self, token: u16, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        assert_eq!(token, 0, "expected CALLT token 0");
        stack.clear();
        stack.push(StackValue::Array(vec![StackValue::ByteString(
            b"Hello World!".to_vec(),
        )]));
        Ok(())
    }
}
