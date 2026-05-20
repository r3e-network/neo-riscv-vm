use neo_riscv_host::execute_script;
use std::time::{Duration, Instant};

const PUSHINT8: u8 = 0x00;
const PUSH1: u8 = 0x11;
const PUSH2: u8 = 0x12;
const NOP: u8 = 0x21;
const RET: u8 = 0x40;
const DROP: u8 = 0x45;
const DUP: u8 = 0x4a;
const ADD: u8 = 0x9e;
const NEWARRAY0: u8 = 0xc2;
const NEWARRAY: u8 = 0xc3;
const APPEND: u8 = 0xcf;
const SETITEM: u8 = 0xd0;

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
        "ret" => vec![RET],
        "nop100" => {
            let mut script = vec![NOP; 100];
            script.push(RET);
            script
        }
        "arithmetic" => {
            let mut script = Vec::new();
            for _ in 0..250 {
                script.extend_from_slice(&[PUSH1, PUSH2, ADD, DROP]);
            }
            script.push(RET);
            script
        }
        "setitem" => {
            let mut script = vec![PUSHINT8, 100, NEWARRAY];
            for i in 0..50 {
                script.extend_from_slice(&[DUP, PUSHINT8, i as u8, PUSHINT8, 42, SETITEM]);
            }
            script.push(RET);
            script
        }
        "append" => {
            let mut script = vec![NEWARRAY0];
            for i in 0..100 {
                script.extend_from_slice(&[DUP, PUSHINT8, i as u8, APPEND]);
            }
            script.push(RET);
            script
        }
        other => {
            panic!("unknown mode {other}; expected empty, ret, nop100, arithmetic, setitem, append")
        }
    }
}
