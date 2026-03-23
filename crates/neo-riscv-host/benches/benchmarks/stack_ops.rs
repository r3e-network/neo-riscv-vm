use criterion::{black_box, Criterion};
use neo_riscv_host::execute_script;

pub fn bench(c: &mut Criterion) {
    let mut script = Vec::new();
    for _ in 0..250 {
        script.extend_from_slice(&[0x11, 0x4a, 0x50, 0x45]); // PUSH1, DUP, SWAP, DROP
    }
    script.push(0x40); // RET

    c.bench_function("stack_manipulation_1000_ops", |b| {
        b.iter(|| {
            execute_script(black_box(&script)).expect("stack benchmark script should execute")
        })
    });
}
