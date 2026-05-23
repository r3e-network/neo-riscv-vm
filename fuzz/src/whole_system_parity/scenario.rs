use neo_riscv_abi::{ExecutionResult, StackValue};
use neo_riscv_fuzz::SimpleRng;

#[cfg(test)]
use neo_riscv_abi::VmState;

use super::{
    build_runtime_platform_script, build_storage_round_trip_script, runtime_platform_api,
    seed_from_bytes, seeded_bytes, seeded_bytes_from_rng, storage_get_api,
    storage_get_context_api, storage_put_api, ScenarioKind, TraceEntry, PLATFORM_RESULT,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scenario {
    pub(crate) kind: ScenarioKind,
    pub(crate) context_token: Vec<u8>,
    pub(crate) key: Vec<u8>,
    pub(crate) value: Vec<u8>,
}

impl Scenario {
    pub(crate) fn from_fuzz_input(data: &[u8]) -> Self {
        let seed = seed_from_bytes(data);
        if data.first().copied().unwrap_or(0) & 1 == 0 {
            Self::platform_from_seed(seed)
        } else {
            Self::storage_from_seed(seed)
        }
    }

    pub(crate) fn platform_from_seed(seed: u64) -> Self {
        Self {
            kind: ScenarioKind::Platform,
            context_token: seeded_bytes(seed, 6),
            key: vec![b'k'],
            value: vec![b'v'],
        }
    }

    pub(crate) fn storage_from_seed(seed: u64) -> Self {
        let mut rng = SimpleRng::new(seed ^ 0x4e45_4f56_4d52_4953);
        let token_len = 1 + (rng.next() % 4) as usize;
        let key_len = 1 + (rng.next() % 4) as usize;
        let value_len = 1 + (rng.next() % 6) as usize;
        let mut context_token = b"NRSC".to_vec();
        context_token.extend(seeded_bytes_from_rng(&mut rng, token_len));

        Self {
            kind: ScenarioKind::StorageRoundTrip,
            context_token,
            key: seeded_bytes_from_rng(&mut rng, key_len),
            value: seeded_bytes_from_rng(&mut rng, value_len),
        }
    }

    pub(crate) fn script(&self) -> Vec<u8> {
        match self.kind {
            ScenarioKind::Platform => build_runtime_platform_script(),
            ScenarioKind::StorageRoundTrip => {
                build_storage_round_trip_script(&self.key, &self.value)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn expected_trace(&self) -> Vec<TraceEntry> {
        match self.kind {
            ScenarioKind::Platform => vec![TraceEntry {
                api: runtime_platform_api(),
                ip: 0,
                stack: Vec::new(),
            }],
            ScenarioKind::StorageRoundTrip => vec![
                TraceEntry {
                    api: storage_get_context_api(),
                    ip: 0,
                    stack: Vec::new(),
                },
                TraceEntry {
                    api: storage_put_api(),
                    ip: 10 + self.key.len() + self.value.len(),
                    stack: vec![
                        StackValue::ByteString(self.context_token.clone()),
                        StackValue::ByteString(self.key.clone()),
                        StackValue::ByteString(self.value.clone()),
                    ],
                },
                TraceEntry {
                    api: storage_get_api(),
                    ip: 17 + (2 * self.key.len()) + self.value.len(),
                    stack: vec![
                        StackValue::ByteString(self.context_token.clone()),
                        StackValue::ByteString(self.key.clone()),
                    ],
                },
            ],
        }
    }

    #[cfg(test)]
    pub(crate) fn expected_result(&self) -> ExecutionResult {
        ExecutionResult {
            fee_consumed_pico: 0,
            state: VmState::Halt,
            stack: match self.kind {
                ScenarioKind::Platform => vec![StackValue::ByteString(PLATFORM_RESULT.to_vec())],
                ScenarioKind::StorageRoundTrip => {
                    vec![StackValue::ByteString(self.value.clone())]
                }
            },
            fault_message: None,
        }
    }
}
