use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct InstructionPointerHost {
    pub(crate) observed_ip: Option<usize>,
}

impl SyscallProvider for InstructionPointerHost {
    fn syscall(&mut self, api: u32, ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        if api != neo_riscv_abi::interop_hash("System.Contract.CallNative") {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.observed_ip = Some(ip);
        stack.pop();
        Ok(())
    }
}
