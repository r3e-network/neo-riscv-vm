#!/bin/bash
set -e

echo "=== Deploy & Invoke Test ==="

BINARY="examples/counter/target/counter.polkavm"

# 1. Generate manifest
echo "Generating manifest..."
./scripts/generate-manifest.sh Counter manifest.json

# 2. Package
echo "Packaging contract..."
./scripts/package-contract.sh "$BINARY" manifest.json contract.nef

# 3. Build deployment request
echo "Deploying to testnet..."
./scripts/deploy-contract.sh contract.nef testnet

# 4. Build invocation request
echo "Invoking contract..."
./scripts/invoke-contract.sh 0x123 increment

echo "✓ Deploy & invoke test passed"
