#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code))]

extern crate libfuzzer_sys;

use std::collections::BTreeMap;

use neo_riscv_abi::{interop_hash, ExecutionResult, StackValue};
use neo_riscv_guest::{interpret_with_stack_and_syscalls, SyscallProvider};
use neo_riscv_host::{execute_script_with_host_and_stack, HostCallbackResult, RuntimeContext};

const PLATFORM_RESULT: &[u8] = b"NEO";

#[cfg(not(test))]
use libfuzzer_sys::fuzz_target;

#[cfg(not(test))]
fuzz_target!(|data: &[u8]| {
    let scenario = Scenario::from_fuzz_input(data);
    assert_parity(&scenario);
});

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParityRun {
    outcome: Result<ExecutionResult, String>,
    trace: Vec<TraceEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceEntry {
    api: u32,
    ip: usize,
    stack: Vec<StackValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScenarioKind {
    Platform,
    StorageRoundTrip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scenario {
    kind: ScenarioKind,
    context_token: Vec<u8>,
    key: Vec<u8>,
    value: Vec<u8>,
}

impl Scenario {
    fn from_fuzz_input(data: &[u8]) -> Self {
        let seed = seed_from_bytes(data);
        if data.first().copied().unwrap_or(0) & 1 == 0 {
            Self::platform_from_seed(seed)
        } else {
            Self::storage_from_seed(seed)
        }
    }

    fn platform_from_seed(seed: u64) -> Self {
        Self {
            kind: ScenarioKind::Platform,
            context_token: seeded_bytes(seed, 6),
            key: vec![b'k'],
            value: vec![b'v'],
        }
    }

    fn storage_from_seed(seed: u64) -> Self {
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

    fn script(&self) -> Vec<u8> {
        match self.kind {
            ScenarioKind::Platform => build_runtime_platform_script(),
            ScenarioKind::StorageRoundTrip => {
                build_storage_round_trip_script(&self.key, &self.value)
            }
        }
    }

    #[cfg(test)]
    fn expected_trace(&self) -> Vec<TraceEntry> {
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
    fn expected_result(&self) -> ExecutionResult {
        ExecutionResult {
            fee_consumed_pico: 0,
            state: VmState::Halt,
            stack: match self.kind {
                ScenarioKind::Platform => {
                    vec![StackValue::ByteString(PLATFORM_RESULT.to_vec())]
                }
                ScenarioKind::StorageRoundTrip => {
                    vec![StackValue::ByteString(self.value.clone())]
                }
            },
            fault_message: None,
        }
    }
}

struct GuestParityProvider {
    model: DeterministicModel,
}

impl GuestParityProvider {
    fn new(scenario: Scenario) -> Self {
        Self {
            model: DeterministicModel::new(scenario),
        }
    }
}

impl SyscallProvider for GuestParityProvider {
    fn syscall(&mut self, api: u32, ip: usize, stack: &mut Vec<StackValue>) -> Result<(), String> {
        let next_stack = self.model.handle(api, ip, stack.as_slice())?;
        *stack = next_stack;
        Ok(())
    }
}

struct DeterministicModel {
    scenario: Scenario,
    trace: Vec<TraceEntry>,
    storage: BTreeMap<(Vec<u8>, Vec<u8>), Vec<u8>>,
}

impl DeterministicModel {
    fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            trace: Vec::new(),
            storage: BTreeMap::new(),
        }
    }

    fn handle(&mut self, api: u32, ip: usize, stack: &[StackValue]) -> Result<Vec<StackValue>, String> {
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

fn assert_parity(scenario: &Scenario) {
    let _ = run_parity(scenario);
}

fn run_parity(scenario: &Scenario) -> ParityRun {
    let guest = run_direct_guest(scenario);
    let host = run_host_path(scenario);

    assert_eq!(
        guest.trace, host.trace,
        "callback trace mismatch for scenario {scenario:?}\nguest={guest:#?}\nhost={host:#?}"
    );
    assert_eq!(
        guest.outcome, host.outcome,
        "execution result mismatch for scenario {scenario:?}\nguest={guest:#?}\nhost={host:#?}"
    );

    host
}

fn run_direct_guest(scenario: &Scenario) -> ParityRun {
    let script = scenario.script();
    let mut provider = GuestParityProvider::new(scenario.clone());
    let outcome = interpret_with_stack_and_syscalls(&script, Vec::new(), &mut provider);

    ParityRun {
        outcome,
        trace: provider.model.trace,
    }
}

fn run_host_path(scenario: &Scenario) -> ParityRun {
    let script = scenario.script();
    let mut model = DeterministicModel::new(scenario.clone());
    let outcome = execute_script_with_host_and_stack(
        &script,
        Vec::new(),
        default_context(),
        |api, ip, _context, stack| {
            let next_stack = model.handle(api, ip, stack)?;
            Ok(HostCallbackResult { stack: next_stack })
        },
    );

    ParityRun {
        outcome,
        trace: model.trace,
    }
}

fn default_context() -> RuntimeContext {
    RuntimeContext {
        trigger: 0x40,
        network: 0,
        address_version: 53,
        timestamp: None,
        gas_left: 0,
        exec_fee_factor_pico: 0,
    }
}

fn build_runtime_platform_script() -> Vec<u8> {
    let mut script = Vec::new();
    script.push(0x41);
    script.extend_from_slice(&runtime_platform_api().to_le_bytes());
    script.push(0x40);
    script
}

fn build_storage_round_trip_script(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut script = Vec::new();
    script.push(0x41);
    script.extend_from_slice(&storage_get_context_api().to_le_bytes());
    script.push(0x4a);
    push_data(&mut script, key);
    push_data(&mut script, value);
    script.push(0x41);
    script.extend_from_slice(&storage_put_api().to_le_bytes());
    push_data(&mut script, key);
    script.push(0x41);
    script.extend_from_slice(&storage_get_api().to_le_bytes());
    script.push(0x40);
    script
}

fn push_data(script: &mut Vec<u8>, bytes: &[u8]) {
    assert!(bytes.len() <= u8::MAX as usize, "PUSHDATA1 payload too large");
    script.push(0x0c);
    script.push(bytes.len() as u8);
    script.extend_from_slice(bytes);
}

fn stack_bytes(value: &StackValue, label: &str) -> Result<Vec<u8>, String> {
    match value {
        StackValue::ByteString(bytes) | StackValue::Buffer(bytes) => Ok(bytes.clone()),
        other => Err(format!("expected {label} as bytes, got {other:?}")),
    }
}

fn runtime_platform_api() -> u32 {
    interop_hash("System.Runtime.Platform")
}

fn storage_get_context_api() -> u32 {
    interop_hash("System.Storage.GetContext")
}

fn storage_put_api() -> u32 {
    interop_hash("System.Storage.Put")
}

fn storage_get_api() -> u32 {
    interop_hash("System.Storage.Get")
}

fn seed_from_bytes(data: &[u8]) -> u64 {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64 ^ data.len() as u64;
    for &byte in data {
        seed ^= byte as u64;
        seed = seed.rotate_left(7).wrapping_mul(0x5851_f42d_4c95_7f2d);
    }
    seed
}

fn seeded_bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = SimpleRng::new(seed);
    seeded_bytes_from_rng(&mut rng, len)
}

fn seeded_bytes_from_rng(rng: &mut SimpleRng, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| ((rng.next() >> 24) & 0xff) as u8)
        .collect()
}

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_seed_matches_direct_guest_and_host_path() {
        let scenario = Scenario::platform_from_seed(0x5150_4c41_5446_4f52);
        let run = run_parity(&scenario);
        assert_eq!(run.trace, scenario.expected_trace());
        assert_eq!(run.outcome, Ok(scenario.expected_result()));
    }

    #[test]
    fn storage_seed_matches_direct_guest_and_host_path() {
        let scenario = Scenario::storage_from_seed(0x5354_4f52_4147_4531);
        let run = run_parity(&scenario);
        assert_eq!(run.trace, scenario.expected_trace());
        assert_eq!(run.outcome, Ok(scenario.expected_result()));
    }
}
