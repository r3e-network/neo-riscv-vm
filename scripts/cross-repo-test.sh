#!/bin/bash
set -e

# Cross-repository test orchestration
# Tests neo-riscv-vm → neo-riscv-core → neo-riscv-node integration

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR/.."
CORE_DIR="${CORE_DIR:-$HOME/git/neo-riscv-core}"
NODE_DIR="${NODE_DIR:-$HOME/git/neo-riscv-node}"

echo "=== Cross-Repo Test Orchestration ==="
echo "VM:   $VM_DIR"
echo "Core: $CORE_DIR"
echo "Node: $NODE_DIR"

# Build VM
echo "Building neo-riscv-vm..."
cd "$VM_DIR"
cargo build --release

# Build Core
echo "Building neo-riscv-core..."
cd "$CORE_DIR"
dotnet build -c Release

# Build Node
echo "Building neo-riscv-node..."
cd "$NODE_DIR"
dotnet build -c Release

echo "✓ All builds complete"
