use criterion::{black_box, Criterion};
use neo_riscv_host::execute_script;

pub fn bench(c: &mut Criterion) {
    let mut script = Vec::new();
    for _ in 0..500 {
        script.extend_from_slice(&[0x08, 0x24, 0x02, 0x11]); // PUSHT, JMPIF +2, PUSH1
    }
    script.push(0x40); // RET

    c.bench_function("control_flow_500_jumps", |b| {
        b.iter(|| {
            execute_script(black_box(&script))
                .expect("control-flow benchmark script should execute")
        })
    });
}
