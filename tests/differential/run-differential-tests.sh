#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <neovm_state.json> <riscv_state.json> [neovm_gas] [riscv_gas]" >&2
  exit 1
fi

NEOVM_STATE="$1"
RISCV_STATE="$2"

python "${ROOT_DIR}/tests/differential/assertions.py" "${NEOVM_STATE}" "${RISCV_STATE}"

if [[ $# -ge 4 ]]; then
  python "${ROOT_DIR}/tests/differential/gas_assertions.py" "$3" "$4"
fi

echo "Differential assertions completed successfully."
