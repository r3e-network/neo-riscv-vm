#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST_LIB="${ROOT_DIR}/target/debug/libneo_riscv_host.so"
NEO_TEST_PROJECT="/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj"

cargo test -p neo-riscv-guest -p neo-riscv-host
cargo build -p neo-riscv-host

NEO_RISCV_HOST_LIB="${HOST_LIB}" \
dotnet test "${NEO_TEST_PROJECT}"
