use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct PlatformHost;

impl SyscallProvider for PlatformHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if api == neo_riscv_abi::interop_hash("System.Runtime.Platform") {
            stack.push(StackValue::ByteString(b"NEO".to_vec()));
            Ok(())
        } else {
            Err(format!("unexpected syscall 0x{api:08x}"))
        }
    }
}
