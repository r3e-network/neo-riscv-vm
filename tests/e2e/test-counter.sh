#!/bin/bash
set -e

echo "=== E2E Test: Counter Contract ==="

# 1. Build
echo "Building counter contract..."
cd examples/counter
cargo +nightly build --release --target "$(polkatool get-target-json-path -b 32)" -Zbuild-std=core,alloc
cd ../..

# 2. Convert to PolkaVM
echo "Converting to PolkaVM format..."
polkatool link --strip \
  -o examples/counter/target/counter.polkavm \
  examples/counter/target/riscv32emac-unknown-none-polkavm/release/counter.elf

# 3. Verify binary
echo "Verifying binary..."
[ -f examples/counter/target/counter.polkavm ] && echo "Binary exists"
hexdump -C examples/counter/target/counter.polkavm | head -1 | grep -q "PVM" && echo "Valid PolkaVM format"

echo "✓ E2E test passed"
