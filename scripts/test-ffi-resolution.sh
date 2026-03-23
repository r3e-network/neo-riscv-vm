#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_PATH="${NEO_RISCV_HOST_LIB:-${ROOT_DIR}/target/debug/libneo_riscv_host.so}"

echo "Testing FFI library resolution..."

if [[ ! -f "${LIB_PATH}" ]]; then
  cargo build -p neo-riscv-host
fi

if [[ ! -f "${LIB_PATH}" ]]; then
  echo "Host library not found: ${LIB_PATH}" >&2
  exit 1
fi

python3 - "${LIB_PATH}" <<'PY'
import ctypes
import sys

path = sys.argv[1]
ctypes.CDLL(path)
print(f"Loaded {path}")
PY

echo "✓ FFI library loaded successfully"
