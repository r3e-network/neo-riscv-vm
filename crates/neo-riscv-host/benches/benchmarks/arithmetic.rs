use criterion::{Criterion, black_box};
use neo_riscv_abi::OpCode;
use neo_riscv_host::execute_script;

pub fn bench(c: &mut Criterion) {
    let mut script = Vec::new();
    for _ in 0..250 {
        script.extend_from_slice(&[
            OpCode::PUSH1.byte(),
            OpCode::PUSH2.byte(),
            OpCode::ADD.byte(),
        ]);
        script.push(OpCode::DROP.byte());
    }
    script.push(OpCode::RET.byte());

    c.bench_function("arithmetic_1000_ops", |b| {
        b.iter(|| {
            execute_script(black_box(&script)).expect("arithmetic benchmark script should execute")
        })
    });
}
