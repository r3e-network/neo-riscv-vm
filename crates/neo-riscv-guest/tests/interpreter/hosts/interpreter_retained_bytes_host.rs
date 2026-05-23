use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

pub(crate) struct InterpreterRetainedBytesHost<'a> {
    pub(crate) observed_log: &'a mut Option<Vec<StackValue>>,
    pub(crate) observed_get: &'a mut Option<Vec<StackValue>>,
}

impl SyscallProvider for InterpreterRetainedBytesHost<'_> {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let platform_api = neo_riscv_abi::interop_hash("System.Runtime.Platform");
        let log_api = neo_riscv_abi::interop_hash("System.Runtime.Log");
        let local_get_api = neo_riscv_abi::interop_hash("System.Storage.Local.Get");

        if api == platform_api {
            *stack = vec![StackValue::ByteString(b"v".to_vec())];
            return Ok(());
        }
        if api == log_api {
            *self.observed_log = Some(stack.clone());
            stack.clear();
            return Ok(());
        }
        if api == local_get_api {
            *self.observed_get = Some(stack.clone());
            *stack = vec![StackValue::Null];
            return Ok(());
        }
        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}
