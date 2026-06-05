use criterion::{Criterion, criterion_group, criterion_main};

mod benchmarks;

fn benchmark_suite(c: &mut Criterion) {
    benchmarks::overhead::bench(c);
    benchmarks::arithmetic::bench(c);
    benchmarks::codec::bench(c);
    benchmarks::control_flow::bench(c);
    benchmarks::stack_ops::bench(c);
}

criterion_group!(benches, benchmark_suite);
criterion_main!(benches);
