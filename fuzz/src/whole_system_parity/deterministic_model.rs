use std::collections::BTreeMap;

use neo_riscv_abi::StackValue;

use super::{
    runtime_platform_api, stack_bytes, storage_get_api, storage_get_context_api, storage_put_api,
    Scenario, ScenarioKind, TraceEntry, PLATFORM_RESULT,
};

pub(crate) struct DeterministicModel {
    scenario: Scenario,
    pub(crate) trace: Vec<TraceEntry>,
    storage: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>,
}

impl DeterministicModel {
    pub(crate) fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            trace: Vec::new(),
            storage: BTreeMap::new(),
        }
    }

    pub(crate) fn handle(
        &mut self,
        api: u32,
        ip: usize,
        stack: &[StackValue],
    ) -> Result<Vec<StackValue>, String> {
        self.trace.push(TraceEntry {
            api,
            ip,
            stack: stack.to_vec(),
        });

        match self.scenario.kind {
            ScenarioKind::Platform => self.handle_platform(api, stack),
            ScenarioKind::StorageRoundTrip => self.handle_storage(api, stack),
        }
    }

    fn handle_platform(&mut self, api: u32, stack: &[StackValue]) -> Result<Vec<StackValue>, String> {
        if api != runtime_platform_api() {
            return Err(format!("unexpected platform syscall 0x{api:08x}"));
        }
        if !stack.is_empty() {
            return Err(format!("platform syscall expected empty stack, got {stack:?}"));
        }

        Ok(vec![StackValue::ByteString(PLATFORM_RESULT.to_vec())])
    }

    fn handle_storage(&mut self, api: u32, stack: &[StackValue]) -> Result<Vec<StackValue>, String> {
        match api {
            value if value == storage_get_context_api() => {
                if !stack.is_empty() {
                    return Err(format!(
                        "storage get-context expected empty stack, got {stack:?}"
                    ));
                }

                Ok(vec![StackValue::ByteString(self.scenario.context_token.clone())])
            }
            value if value == storage_put_api() => {
                if stack.len() != 3 {
                    return Err(format!("storage put expected 3 args, got {stack:?}"));
                }

                let context = stack_bytes(&stack[0], "storage context token")?;
                let key = stack_bytes(&stack[1], "storage key")?;
                let value = stack_bytes(&stack[2], "storage value")?;
                self.storage.insert((context, key), value);
                Ok(Vec::new())
            }
            value if value == storage_get_api() => {
                if stack.len() != 2 {
                    return Err(format!("storage get expected 2 args, got {stack:?}"));
                }

                let context = stack_bytes(&stack[0], "storage context token")?;
                let key = stack_bytes(&stack[1], "storage key")?;
                Ok(vec![
                    self.storage
                        .get(&(context, key))
                        .cloned()
                        .map(StackValue::ByteString)
                        .unwrap_or(StackValue::Null),
                ])
            }
            _ => Err(format!("unexpected storage syscall 0x{api:08x}")),
        }
    }
}
