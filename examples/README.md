# RISC-V Contract Examples

## Examples

1. **counter** - Simple counter with increment/get/reset
2. **storage** - Storage operations demo
3. **hello-world** - Minimal contract
4. **nep17-token** - NEP-17 token implementation

## Build & Test

```bash
# Compile an example to a PolkaVM contract blob
./scripts/compile-riscv-contract.sh examples/counter examples/counter/target/counter.polkavm

# Run the example E2E flow
./tests/e2e/run-all.sh
```

## Deploy

```bash
./scripts/package-contract.sh examples/counter/target/counter.polkavm manifest.json contract.nef
./scripts/deploy-contract.sh contract.nef testnet
```
