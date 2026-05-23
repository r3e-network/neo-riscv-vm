use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

#[derive(Default)]
pub(crate) struct ConsecutiveLargeCallHost {
    pub(crate) call_count: i64,
}

impl SyscallProvider for ConsecutiveLargeCallHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let expected_api = neo_riscv_abi::interop_hash("System.Contract.Call");
        if api != expected_api {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.call_count += 1;
        *stack = vec![StackValue::Integer(self.call_count)];
        Ok(())
    }
}
