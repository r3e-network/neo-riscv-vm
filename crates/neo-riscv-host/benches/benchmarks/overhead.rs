use criterion::{black_box, Criterion};
use neo_riscv_abi::OpCode;
use neo_riscv_host::execute_script;

pub fn bench(c: &mut Criterion) {
    c.bench_function("host_overhead_empty_script", |b| {
        b.iter(|| execute_script(black_box(&[])).expect("empty script should execute"))
    });

    c.bench_function("host_overhead_ret_only", |b| {
        b.iter(|| {
            execute_script(black_box(&[OpCode::RET.byte()]))
                .expect("RET-only script should execute")
        })
    });

    c.bench_function("host_overhead_push_drop_ret", |b| {
        b.iter(|| {
            execute_script(black_box(&[
                OpCode::PUSH1.byte(),
                OpCode::DROP.byte(),
                OpCode::RET.byte(),
            ]))
            .expect("PUSH/DROP/RET script should execute")
        })
    });

    let mut nops = vec![OpCode::NOP.byte(); 100];
    nops.push(OpCode::RET.byte());
    c.bench_function("guest_nop_100_ops", |b| {
        b.iter(|| execute_script(black_box(&nops)).expect("NOP benchmark should execute"))
    });
}
