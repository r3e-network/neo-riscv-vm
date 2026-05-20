use criterion::{black_box, Criterion};
use neo_riscv_host::execute_script;

const NOP: u8 = 0x21;
const RET: u8 = 0x40;
const PUSH1: u8 = 0x11;
const DROP: u8 = 0x45;

pub fn bench(c: &mut Criterion) {
    c.bench_function("host_overhead_empty_script", |b| {
        b.iter(|| execute_script(black_box(&[])).expect("empty script should execute"))
    });

    c.bench_function("host_overhead_ret_only", |b| {
        b.iter(|| execute_script(black_box(&[RET])).expect("RET-only script should execute"))
    });

    c.bench_function("host_overhead_push_drop_ret", |b| {
        b.iter(|| {
            execute_script(black_box(&[PUSH1, DROP, RET]))
                .expect("PUSH/DROP/RET script should execute")
        })
    });

    let mut nops = vec![NOP; 100];
    nops.push(RET);
    c.bench_function("guest_nop_100_ops", |b| {
        b.iter(|| execute_script(black_box(&nops)).expect("NOP benchmark should execute"))
    });
}
