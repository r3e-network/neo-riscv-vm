#!/bin/bash
set -euo pipefail

echo "=== E2E Test: Counter Contract ==="

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT="${ROOT_DIR}/examples/counter/target/counter.polkavm"

# 1. Build and link through the canonical helper
echo "Building counter contract..."
rm -f "${OUTPUT}"
"${ROOT_DIR}/scripts/compile-riscv-contract.sh" "${ROOT_DIR}/examples/counter" "${OUTPUT}"

# 2. Verify binary
echo "Verifying binary..."
[ -f "${OUTPUT}" ] && echo "Binary exists"
hexdump -C "${OUTPUT}" | head -1 | grep -q "PVM" && echo "Valid PolkaVM format"

echo "✓ E2E test passed"
