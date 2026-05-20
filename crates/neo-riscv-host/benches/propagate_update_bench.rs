use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neo_riscv_host::execute_script;

const PUSHINT8: u8 = 0x00;
const RET: u8 = 0x40;
const DUP: u8 = 0x4a;
const NEWARRAY0: u8 = 0xc2;
const NEWARRAY: u8 = 0xc3;
const APPEND: u8 = 0xcf;
const SETITEM: u8 = 0xd0;

fn bench_setitem_operations(c: &mut Criterion) {
    // Script: create array with 100 elements, then SETITEM 50 times
    let mut script = vec![
        PUSHINT8, 100, NEWARRAY, // NEWARRAY 100
    ];

    // Push 50 SETITEM operations
    for i in 0..50 {
        script.extend_from_slice(&[DUP, PUSHINT8, i as u8, PUSHINT8, 42, SETITEM]);
    }
    script.push(RET);

    c.bench_function("setitem_50_ops_on_100_elem_array", |b| {
        b.iter(|| execute_script(black_box(&script)).expect("SETITEM benchmark should execute"))
    });
}

fn bench_append_operations(c: &mut Criterion) {
    // Script: create empty array, then APPEND 100 times
    let mut script = vec![NEWARRAY0];

    for i in 0..100 {
        script.extend_from_slice(&[DUP, PUSHINT8, i as u8, APPEND]);
    }
    script.push(RET);

    c.bench_function("append_100_ops", |b| {
        b.iter(|| execute_script(black_box(&script)).expect("APPEND benchmark should execute"))
    });
}

criterion_group!(benches, bench_setitem_operations, bench_append_operations);
criterion_main!(benches);
