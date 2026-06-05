use criterion::{Criterion, black_box};
use neo_riscv_abi::OpCode;
use neo_riscv_host::execute_script;

pub fn bench(c: &mut Criterion) {
    let mut script = Vec::new();
    for _ in 0..500 {
        script.extend_from_slice(&[
            OpCode::PUSHT.byte(),
            OpCode::JMPIF.byte(),
            0x02,
            OpCode::PUSH1.byte(),
        ]);
    }
    script.push(OpCode::RET.byte());

    c.bench_function("control_flow_500_jumps", |b| {
        b.iter(|| {
            execute_script(black_box(&script))
                .expect("control-flow benchmark script should execute")
        })
    });
}
