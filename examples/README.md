# RISC-V Contract Examples

## Examples

1. **counter** - Simple counter with increment/get/reset
2. **storage** - Storage operations demo
3. **hello-world** - Minimal contract
4. **nep17-token** - NEP-17 token implementation

## Build & Test

```bash
# Build example
cd examples/counter
cargo build --release

# Run E2E tests
../../tests/e2e/run-all.sh
```

## Deploy

```bash
./scripts/neo-riscv-cli.sh deploy contract.nef testnet
```
