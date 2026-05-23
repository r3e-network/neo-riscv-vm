use neo_riscv_abi::OpCode;
use neo_riscv_host::execute_script;
use std::time::{Duration, Instant};

fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "append".to_string());
    let duration = std::env::var("PROFILE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(20));
    let script = script_for(&mode);

    let start = Instant::now();
    let mut iterations = 0u64;
    while start.elapsed() < duration {
        execute_script(&script).expect("profile script should execute");
        iterations += 1;
    }

    eprintln!(
        "profile_hotspot mode={mode} iterations={iterations} elapsed_ms={}",
        start.elapsed().as_millis()
    );
}

fn script_for(mode: &str) -> Vec<u8> {
    match mode {
        "empty" => Vec::new(),
        "ret" => vec![OpCode::RET.byte()],
        "nop100" => {
            let mut script = vec![OpCode::NOP.byte(); 100];
            script.push(OpCode::RET.byte());
            script
        }
        "arithmetic" => {
            let mut script = Vec::new();
            for _ in 0..250 {
                script.extend_from_slice(&[
                    OpCode::PUSH1.byte(),
                    OpCode::PUSH2.byte(),
                    OpCode::ADD.byte(),
                    OpCode::DROP.byte(),
                ]);
            }
            script.push(OpCode::RET.byte());
            script
        }
        "setitem" => {
            let mut script = vec![OpCode::PUSHINT8.byte(), 100, OpCode::NEWARRAY.byte()];
            for i in 0..50 {
                script.extend_from_slice(&[
                    OpCode::DUP.byte(),
                    OpCode::PUSHINT8.byte(),
                    i as u8,
                    OpCode::PUSHINT8.byte(),
                    42,
                    OpCode::SETITEM.byte(),
                ]);
            }
            script.push(OpCode::RET.byte());
            script
        }
        "append" => {
            let mut script = vec![OpCode::NEWARRAY0.byte()];
            for i in 0..100 {
                script.extend_from_slice(&[
                    OpCode::DUP.byte(),
                    OpCode::PUSHINT8.byte(),
                    i as u8,
                    OpCode::APPEND.byte(),
                ]);
            }
            script.push(OpCode::RET.byte());
            script
        }
        other => {
            panic!("unknown mode {other}; expected empty, ret, nop100, arithmetic, setitem, append")
        }
    }
}
