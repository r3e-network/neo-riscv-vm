#!/bin/bash
# Plugin loading validation test

CORE_DIR="${CORE_DIR:-$HOME/git/neo-riscv-core}"
ADAPTER_DIR="$CORE_DIR/../neo-riscv-vm/compat/Neo.Riscv.Adapter"

echo "Testing plugin loading..."

# Check adapter builds
cd "$ADAPTER_DIR"
dotnet build -c Release

# Check plugin can be discovered
# TODO: Add actual plugin loading test with Neo.CLI

echo "✓ Plugin loading validation placeholder"
