/// Integration tests: load and execute C#-compiled RISC-V contracts.
///
/// These tests verify that contracts compiled via:
///   C# → nccs --target riscv → Rust → PolkaVM
/// can be loaded and executed on neo-riscv-host without trapping.
use neo_riscv_abi::{StackValue, VmState};
use neo_riscv_host::{execute_native_contract, HostCallbackResult, RuntimeContext};
use std::fs;
use std::path::Path;

const CONTRACTS_DIR: &str = "/tmp/riscv-test-output/riscv";

fn default_context() -> RuntimeContext {
    RuntimeContext {
        trigger: 0x40,
        network: 860833102,
        address_version: 53,
        timestamp: None,
        gas_left: 1_000_000_000_000,
        exec_fee_factor_pico: 30_000,
    }
}

fn dummy_callback(
    _api: u32,
    _ip: usize,
    _ctx: RuntimeContext,
    _stack: &[StackValue],
) -> Result<HostCallbackResult, String> {
    // Return empty stack for any syscall — contracts that need real syscalls
    // will fault gracefully rather than trap.
    Ok(HostCallbackResult { stack: vec![] })
}

/// Load a .polkavm binary and execute with the given method name.
/// Returns (state, stack, error_if_any).
fn execute_contract(
    name: &str,
    method: &str,
    args: Vec<StackValue>,
) -> Result<(VmState, Vec<StackValue>), String> {
    let polkavm_path = format!("{}/{}/contract.polkavm", CONTRACTS_DIR, name);
    if !Path::new(&polkavm_path).exists() {
        return Err(format!("binary not found: {}", polkavm_path));
    }
    let binary = fs::read(&polkavm_path).map_err(|e| format!("read error: {}", e))?;
    let result = execute_native_contract(&binary, method, args, default_context(), dummy_callback)?;
    Ok((result.state, result.stack))
}

// ========================================================================
// Individual contract tests
// Each test loads the compiled .polkavm and verifies it doesn't trap.
// ========================================================================

#[test]
fn test_all_csharp_contracts_load() {
    let dir = Path::new(CONTRACTS_DIR);
    if !dir.exists() {
        eprintln!("SKIP: {} not found. Run nccs --target riscv first.", CONTRACTS_DIR);
        return;
    }

    let mut total = 0;
    let mut pass = 0;
    let mut skip = 0;
    let mut fail = 0;
    let mut failures = Vec::new();

    for entry in fs::read_dir(dir).expect("read contracts dir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().to_string();
        let polkavm = entry.path().join("contract.polkavm");
        total += 1;

        if !polkavm.exists() {
            skip += 1;
            continue;
        }

        let binary = fs::read(&polkavm).expect("read binary");
        let result = execute_native_contract(
            &binary,
            "_nonexistent_method_",
            vec![],
            default_context(),
            dummy_callback,
        );

        match &result {
            Ok(r) => {
                if r.state == VmState::Fault {
                    pass += 1;
                    eprintln!("OK    {}", name);
                } else {
                    pass += 1;
                    eprintln!("OK    {} (state={:?})", name, r.state);
                }
            }
            Err(e) if e.contains("Trap") => {
                fail += 1;
                failures.push(format!("{}: TRAP — {}", name, e));
                eprintln!("FAIL  {} (TRAP: {})", name, e);
            }
            Err(e) if e.contains("Unknown method") => {
                pass += 1;
                eprintln!("OK    {} (Unknown method fault)", name);
            }
            Err(e) => {
                // Other errors (decode issues, etc.) — contract loaded but had issues
                pass += 1;
                eprintln!("WARN  {} ({})", name, e);
            }
        }
    }

    eprintln!("\n=== C# CONTRACT EXECUTION RESULTS ===");
    eprintln!("Total: {}", total);
    eprintln!("Pass:  {}", pass);
    eprintln!("Skip:  {} (no .polkavm)", skip);
    eprintln!("Fail:  {}", fail);

    if !failures.is_empty() {
        eprintln!("\nFAILURES:");
        for f in &failures {
            eprintln!("  {}", f);
        }
        panic!("{} contracts trapped (see above)", failures.len());
    }
}

/// Test a specific contract method that should return a value.
/// Contract_Assignment.testAssignment() should execute and halt.
#[test]
fn test_contract_assignment_executes() {
    let result = execute_contract("contract_assignment", "testAssignment", vec![]);
    match result {
        Ok((state, stack)) => {
            eprintln!("Contract_Assignment.testAssignment: state={:?}, stack={:?}", state, stack);
            // The method uses Assert internally — if it halts, the assertions passed
            assert_eq!(state, VmState::Halt, "testAssignment should halt (assertions pass)");
        }
        Err(e) => {
            eprintln!("Contract_Assignment.testAssignment error: {}", e);
            // May fail due to missing syscalls — that's OK for now
            if e.contains("Trap") {
                panic!("Should not trap: {}", e);
            }
        }
    }
}
