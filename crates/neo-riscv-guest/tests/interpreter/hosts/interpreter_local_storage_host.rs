use std::collections::HashMap;

use neo_riscv_abi::StackValue;
use neo_riscv_guest::SyscallProvider;

#[derive(Default)]
pub(crate) struct InterpreterLocalStorageHost {
    storage: HashMap<Vec<u8>, Vec<u8>>,
}

impl SyscallProvider for InterpreterLocalStorageHost {
    fn syscall(&mut self, api: u32, _ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let local_put = neo_riscv_abi::interop_hash("System.Storage.Local.Put");
        let local_get = neo_riscv_abi::interop_hash("System.Storage.Local.Get");
        let local_delete = neo_riscv_abi::interop_hash("System.Storage.Local.Delete");

        match api {
            value if value == local_put => {
                assert_eq!(stack.len(), 2);
                let key = match &stack[0] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put key bytes, got {other:?}"),
                };
                let val = match &stack[1] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-put value bytes, got {other:?}"),
                };
                self.storage.insert(key, val);
                stack.clear();
                Ok(())
            }
            value if value == local_get => {
                assert_eq!(stack.len(), 1);
                let key = match &stack[0] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-get key bytes, got {other:?}"),
                };
                let item = self
                    .storage
                    .get(&key)
                    .cloned()
                    .map(StackValue::ByteString)
                    .unwrap_or(StackValue::Null);
                *stack = vec![item];
                Ok(())
            }
            value if value == local_delete => {
                assert_eq!(stack.len(), 1);
                let key = match &stack[0] {
                    StackValue::ByteString(bytes) => bytes.clone(),
                    other => panic!("expected local-delete key bytes, got {other:?}"),
                };
                self.storage.remove(&key);
                stack.clear();
                Ok(())
            }
            other => Err(format!("unexpected syscall 0x{other:08x}")),
        }
    }
}
