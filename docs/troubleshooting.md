# Troubleshooting Guide

## VM Execution Failures

### Symptom: Contract execution returns FAULT

**Check:**

1. Review fault message in ExecutionResult
2. Check logs for stack trace
3. Verify script bytecode is valid NeoVM

**Common causes:**

- Stack underflow (insufficient operands)
- Division by zero
- Invalid opcode
- Gas exhaustion

### Symptom: FFI panic at host boundary

**Check:**

1. Verify libneo_riscv_host.so version matches adapter
2. Check for ABI mismatches
3. Review catch_unwind logs

**Fix:** Rebuild both Rust and C# components

## Performance Issues

### Symptom: Execution >2x slower than native NeoVM

**Check:**

1. Run benchmarks: `cargo bench`
2. Profile with `perf record`
3. Check guest blob is optimized build

**Fix:** Regenerate guest blob with release profile

## Build Failures

### Symptom: Guest module compilation fails

**Fix:**

```bash
rm -rf target/riscv32emac-unknown-none-polkavm
./scripts/regenerate-guest-blob.sh
```

### Symptom: C# adapter compilation fails

**Check:** Neo.Riscv.Adapter references correct Neo.csproj version
