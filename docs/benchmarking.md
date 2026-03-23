# Criterion Benchmark Configuration

## Baseline Recording

Save current performance as baseline:

```bash
cargo bench --bench benchmark_harness -- --save-baseline main
```

## Regression Detection

Compare against baseline:

```bash
cargo bench --bench benchmark_harness -- --baseline main
```

## Performance Thresholds

Target: Within 2x of native NeoVM execution time.

Baseline measurements (reference hardware: 4-core CPU, 16GB RAM):

- arithmetic_1000_ops: ~500μs
- stack_manipulation_1000_ops: ~450μs
- control_flow_500_jumps: ~300μs

## CI Integration

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench --bench benchmark_harness -- --baseline main
```
