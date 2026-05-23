use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

#[derive(Default)]
pub(crate) struct PackedInteropHost {
    deserialize_count: u64,
    pub(crate) observed_aggregate: Option<Vec<StackValue>>,
}

impl SyscallProvider for PackedInteropHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let deserialize_api = neo_riscv_abi::interop_hash("Crypto.Deserialize");
        let aggregate_api = neo_riscv_abi::interop_hash("Crypto.Aggregate");

        if api == deserialize_api {
            self.deserialize_count += 1;
            stack.push(StackValue::Interop(self.deserialize_count));
            return Ok(());
        }

        if api == aggregate_api {
            self.observed_aggregate = Some(stack.clone());
            stack.clear();
            stack.push(StackValue::Interop(99));
            return Ok(());
        }

        Err(format!("unexpected syscall 0x{api:08x}"))
    }
}
