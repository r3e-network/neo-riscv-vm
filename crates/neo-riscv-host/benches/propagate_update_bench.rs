use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neo_riscv_host::execute_script;

fn bench_setitem_operations(c: &mut Criterion) {
    // Script: create array with 100 elements, then SETITEM 50 times
    let mut script = vec![
        0x7A, 0x64, 0x00, // NEWARRAY 100
    ];
    
    // Push 50 SETITEM operations
    for i in 0..50 {
        script.extend_from_slice(&[
            0x78, // DUP
            0x00, i as u8, // PUSH1 index
            0x00, 0xFF, // PUSH1 255
            0xC4, // SETITEM
        ]);
    }
    
    c.bench_function("setitem_50_ops_on_100_elem_array", |b| {
        b.iter(|| {
            execute_script(black_box(&script))
        })
    });
}

fn bench_append_operations(c: &mut Criterion) {
    // Script: create empty array, then APPEND 100 times
    let mut script = vec![
        0xC0, // NEWARRAY0
    ];
    
    for i in 0..100 {
        script.extend_from_slice(&[
            0x78, // DUP
            0x00, i as u8, // PUSH1 value
            0xC8, // APPEND
        ]);
    }
    
    c.bench_function("append_100_ops", |b| {
        b.iter(|| {
            execute_script(black_box(&script))
        })
    });
}

criterion_group!(benches, bench_setitem_operations, bench_append_operations);
criterion_main!(benches);
