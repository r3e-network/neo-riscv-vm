#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODE="${1:-${TEST_MODE:-quick}}"
. "${SCRIPT_DIR}/resolve-host-lib.sh"

if [[ -n "${CARGO_STABLE:-}" ]]; then
  read -r -a CARGO_STABLE_CMD <<< "${CARGO_STABLE}"
else
  CARGO_STABLE_CMD=(cargo)
fi

HOST_LIB="${HOST_LIB:-$(resolve_host_lib "${ROOT_DIR}" release)}"

run_cargo_stable() {
  if [[ -n "${RUSTC_STABLE:-}" ]]; then
    RUSTC="${RUSTC_STABLE}" "${CARGO_STABLE_CMD[@]}" "$@"
  else
    "${CARGO_STABLE_CMD[@]}" "$@"
  fi
}

run_guest_regen() {
  if [[ -n "${RUSTC_NIGHTLY:-}" ]]; then
    RUSTC="${RUSTC_NIGHTLY}" bash "${ROOT_DIR}/scripts/regenerate-guest-blob.sh"
  else
    bash "${ROOT_DIR}/scripts/regenerate-guest-blob.sh"
  fi
}

ensure_release_host() {
  if [[ ! -f "${HOST_LIB}" ]]; then
    run_cargo_stable build -p neo-riscv-host --release
  fi
}

refresh_release_host() {
  run_guest_regen
  run_cargo_stable build -p neo-riscv-host --release
}

run_quick() {
  run_cargo_stable test --workspace --all-targets
  ensure_release_host

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  NEO_RISCV_VM_JSON_MODE=smoke \
  dotnet test "${ROOT_DIR}/dotnet/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj"

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  dotnet test "${ROOT_DIR}/dotnet/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj"

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  "${ROOT_DIR}/scripts/test-ffi-resolution.sh"
}

run_pre_mainnet() {
  run_quick

  python3 -m unittest \
    tests.test_fault_oracle_corpus \
    tests.test_mainnet_corpus_collector \
    tests.test_stateroot_segments

  refresh_release_host

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  dotnet test "${ROOT_DIR}/dotnet/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj" \
    --filter FullyQualifiedName~UT_OpcodeOracleMatrix

  if [[ -f "${ROOT_DIR}/mainnet-validation/logs/neo-cli-launchd.err" ]]; then
    python3 "${ROOT_DIR}/scripts/extract-fault-oracle-corpus.py" \
      --log-file "${ROOT_DIR}/mainnet-validation/logs/neo-cli-launchd.err" \
      --output-dir "${ROOT_DIR}/mainnet-validation/oracle-corpus/faults" \
      --fail-on-reference-halt
  fi
}

run_nightly() {
  run_pre_mainnet

  TIME_PER_TARGET="${TIME_PER_TARGET:-60}" \
  RUNS_PER_TARGET="${RUNS_PER_TARGET:-0}" \
  "${ROOT_DIR}/scripts/run-fuzz-seed-matrix.sh"

  python3 "${ROOT_DIR}/scripts/collect-mainnet-oracle-corpus.py" \
    --start "${MAINNET_CORPUS_START:-0}" \
    --end "${MAINNET_CORPUS_END:-2000}" \
    --output "${ROOT_DIR}/mainnet-validation/oracle-corpus/applicationlogs.jsonl"
}

run_long_running() {
  run_pre_mainnet

  python3 "${ROOT_DIR}/scripts/run-stateroot-segments.py" \
    --jobs "${STATEROOT_SEGMENT_JOBS:-2}"

  if [[ "${RUN_FULL_STATEROOT:-0}" == "1" ]]; then
    "${ROOT_DIR}/scripts/run-mainnet-stateroot-validation.sh"
  else
    echo "[long-running] skipped full mainnet stateroot launch; set RUN_FULL_STATEROOT=1 on a dedicated runner."
  fi
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
Usage: ./scripts/run-all-tests.sh [quick|pre-mainnet|nightly|long-running|full|ci]

Modes:
  quick  Run local VM workspace tests, smoke corpus, adapter tests, and FFI smoke
  pre-mainnet
         Run quick plus oracle utility tests, fresh guest/host rebuild, NeoVM opcode
         type matrix, and local fault-vs-reference extraction when logs exist
  nightly
         Run pre-mainnet plus fuzz seed matrix and reference RPC applicationlog corpus collection
  long-running
         Run pre-mainnet plus configured high-risk state-root segments; set
         RUN_FULL_STATEROOT=1 to start the full mainnet stateroot validator
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
  pre-mainnet)
    run_pre_mainnet
    ;;
  nightly)
    run_nightly
    ;;
  long-running)
    run_long_running
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
