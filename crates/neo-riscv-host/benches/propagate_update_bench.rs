use criterion::{Criterion, black_box, criterion_group, criterion_main};
use neo_riscv_abi::OpCode;
use neo_riscv_host::execute_script;

fn bench_setitem_operations(c: &mut Criterion) {
    // Script: create array with 100 elements, then SETITEM 50 times
    let mut script = vec![OpCode::PUSHINT8.byte(), 100, OpCode::NEWARRAY.byte()];

    // Push 50 SETITEM operations
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

    c.bench_function("setitem_50_ops_on_100_elem_array", |b| {
        b.iter(|| execute_script(black_box(&script)).expect("SETITEM benchmark should execute"))
    });
}

fn bench_append_operations(c: &mut Criterion) {
    // Script: create empty array, then APPEND 100 times
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

    c.bench_function("append_100_ops", |b| {
        b.iter(|| execute_script(black_box(&script)).expect("APPEND benchmark should execute"))
    });
}

criterion_group!(benches, bench_setitem_operations, bench_append_operations);
criterion_main!(benches);
