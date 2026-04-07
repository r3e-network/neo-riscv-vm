use criterion::{black_box, Criterion};
use neo_riscv_abi::{fast_codec, StackValue};

pub fn bench(c: &mut Criterion) {
    let stack = vec![
        StackValue::Integer(42),
        StackValue::ByteString(vec![1, 2, 3, 4, 5]),
        StackValue::Boolean(true),
        StackValue::Array(vec![StackValue::Integer(1), StackValue::Integer(2)]),
    ];

    c.bench_function("fast_codec_encode", |b| {
        b.iter(|| {
            let encoded = fast_codec::encode_stack(black_box(&stack));
            black_box(encoded);
        });
    });

    let encoded = fast_codec::encode_stack(&stack);
    c.bench_function("fast_codec_decode", |b| {
        b.iter(|| {
            let decoded = fast_codec::decode_stack(black_box(&encoded)).unwrap();
            black_box(decoded);
        });
    });
}
