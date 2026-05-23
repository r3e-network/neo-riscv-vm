use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

#[derive(Default)]
pub(crate) struct IntegerRoundTripHost {
    call_count: i64,
    pub(crate) observed_third: Option<Vec<StackValue>>,
}

impl SyscallProvider for IntegerRoundTripHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let expected_api = neo_riscv_abi::interop_hash("System.Test.Multi");
        if api != expected_api {
            return Err(format!("unexpected syscall 0x{api:08x}"));
        }

        self.call_count += 1;
        match self.call_count {
            1 => {
                stack.push(StackValue::Integer(1));
                Ok(())
            }
            2 => {
                *stack = vec![StackValue::Integer(1), StackValue::Integer(2)];
                Ok(())
            }
            3 => {
                self.observed_third = Some(stack.clone());
                Ok(())
            }
            _ => Err("unexpected extra syscall".to_string()),
        }
    }
}
