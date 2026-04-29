#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODE="${1:-${TEST_MODE:-quick}}"

HOST_LIB="${ROOT_DIR}/target/release/libneo_riscv_host.so"

ensure_release_host() {
  if [[ ! -f "${HOST_LIB}" ]]; then
    cargo build -p neo-riscv-host --release
  fi
}

run_quick() {
  cargo test --workspace --all-targets
  ensure_release_host

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  NEO_RISCV_VM_JSON_MODE=smoke \
  dotnet test "${ROOT_DIR}/compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj"

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  dotnet test "${ROOT_DIR}/compat/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj"

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  "${ROOT_DIR}/scripts/test-ffi-resolution.sh"
}

run_local_full() {
  "${ROOT_DIR}/scripts/verify-all.sh"
  "${ROOT_DIR}/tests/e2e/run-all.sh"
  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  "${ROOT_DIR}/scripts/test-ffi-resolution.sh"
}

run_cross_repo_if_available() {
  local core_dir="${CORE_DIR:-${ROOT_DIR}/../neo-riscv-core}"
  local node_dir="${NODE_DIR:-${ROOT_DIR}/../neo-riscv-node}"
  local devpack_dir="${DEVPACK_DIR:-${ROOT_DIR}/../neo-riscv-devpack}"

  if [[ -d "${core_dir}" && -d "${node_dir}" && -d "${devpack_dir}" ]]; then
    "${ROOT_DIR}/scripts/cross-repo-test.sh"
  else
    run_local_full
  fi
}

usage() {
  cat <<'EOF'
Usage: ./scripts/run-all-tests.sh [quick|full|ci]

Modes:
  quick  Run local VM workspace tests, smoke corpus, adapter tests, and FFI smoke
  full   Run the full cross-repo matrix when sibling core/node/devpack repos are available;
         otherwise run the full local VM verification flow
  ci     Run the local full VM verification flow only
EOF
}

cd "${ROOT_DIR}"

case "${MODE}" in
  quick)
    run_quick
    ;;
  full)
    run_cross_repo_if_available
    ;;
  ci)
    run_local_full
    ;;
  --help|-h|help)
    usage
    ;;
  *)
    echo "Unknown mode: ${MODE}" >&2
    usage >&2
    exit 1
    ;;
esac
