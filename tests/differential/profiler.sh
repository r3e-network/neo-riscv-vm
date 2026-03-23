#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT="${DIFFERENTIAL_PROFILE_OUTPUT:-/tmp/riscv_profiling.log}"

if [[ $# -eq 0 ]]; then
  CMD=(cargo test -p neo-riscv-host --test runtime -- --nocapture)
else
  CMD=("$@")
fi

echo "Running performance profiling command: ${CMD[*]}"
/usr/bin/time -v "${CMD[@]}" 2>&1 | tee "${OUTPUT}"
echo "Profiling data written to ${OUTPUT}"
