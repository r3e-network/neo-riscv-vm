#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code))]

extern crate libfuzzer_sys;

mod deterministic_model;
mod guest_parity_provider;
mod parity_run;
mod scenario;
mod trace_entry;

use deterministic_model::DeterministicModel;
use guest_parity_provider::GuestParityProvider;
use neo_riscv_abi::{interop_hash, OpCode, StackValue};
use neo_riscv_fuzz::SimpleRng;
use neo_riscv_guest::interpret_with_stack_and_syscalls;
use neo_riscv_host::{execute_script_with_host_and_stack, HostCallbackResult, RuntimeContext};
use parity_run::ParityRun;
use scenario::Scenario;
use trace_entry::TraceEntry;

#[cfg(test)]
use neo_riscv_abi::VmState;

const PLATFORM_RESULT: &[u8] = b"NEO";

#[cfg(not(test))]
use libfuzzer_sys::fuzz_target;

#[cfg(not(test))]
fuzz_target!(|data: &[u8]| {
    let scenario = Scenario::from_fuzz_input(data);
    assert_parity(&scenario);
});

#[derive(Clone, Debug, PartialEq, Eq)]
enum ScenarioKind {
    Platform,
    StorageRoundTrip,
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
    script.push(OpCode::SYSCALL.byte());
    script.extend_from_slice(&runtime_platform_api().to_le_bytes());
    script.push(OpCode::RET.byte());
    script
}

fn build_storage_round_trip_script(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut script = Vec::new();
    script.push(OpCode::SYSCALL.byte());
    script.extend_from_slice(&storage_get_context_api().to_le_bytes());
    script.push(OpCode::DUP.byte());
    push_data(&mut script, key);
    push_data(&mut script, value);
    script.push(OpCode::SYSCALL.byte());
    script.extend_from_slice(&storage_put_api().to_le_bytes());
    push_data(&mut script, key);
    script.push(OpCode::SYSCALL.byte());
    script.extend_from_slice(&storage_get_api().to_le_bytes());
    script.push(OpCode::RET.byte());
    script
}

fn push_data(script: &mut Vec<u8>, bytes: &[u8]) {
    assert!(bytes.len() <= u8::MAX as usize, "PUSHDATA1 payload too large");
    script.push(OpCode::PUSHDATA1.byte());
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
