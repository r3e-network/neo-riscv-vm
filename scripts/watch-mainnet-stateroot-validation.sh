#!/usr/bin/env bash
# Watch a launchd-backed mainnet state-root replay.
#
# The node logs expected faulted transactions as neo-riscv-fault entries when
# NEO_RISCV_LOG_FAULTS=1. Those are not validation failures by themselves:
# state-root mismatch is the authoritative compatibility signal.
set -euo pipefail
export PATH="/Applications/Codex.app/Contents/Resources:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

LOCAL_RPC="${LOCAL_RPC:-http://127.0.0.1:10332}"
LOG_DIR="${LOG_DIR:-${VM_DIR}/mainnet-validation/logs}"
STATEROOT_LOG="${STATEROOT_LOG:-${LOG_DIR}/stateroot-continuous.tsv}"
CHECKPOINT_FILE="${CHECKPOINT_FILE:-${LOG_DIR}/stateroot-continuous.checkpoint}"
NODE_ERROR_LOG="${NODE_ERROR_LOG:-${LOG_DIR}/neo-cli-launchd.err}"
COMPARATOR_ERROR_LOG="${COMPARATOR_ERROR_LOG:-${LOG_DIR}/stateroot-launchd.err}"
NODE_LABEL="${NODE_LABEL:-com.codex.neo-riscv-node-mainnet}"
COMPARATOR_LABEL="${COMPARATOR_LABEL:-com.codex.neo-riscv-stateroot-mainnet}"
TARGET_HEIGHT="${TARGET_HEIGHT:-0}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-60}"
ONCE=false

FATAL_RE="${FATAL_RE:-integer exceeds i64 range|Unhandled exception|Fatal error|OutOfMemory|StackOverflow|panicked at|thread .* panicked|segmentation fault|Abort trap|core dumped}"

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Options:
  --target HEIGHT       Exit successfully after checkpoint reaches HEIGHT.
  --interval SECONDS   Poll interval. Default: ${INTERVAL_SECONDS}.
  --once               Print one status line and exit.
  --local-rpc URL      Local node RPC URL. Default: ${LOCAL_RPC}.

Environment overrides:
  LOG_DIR, STATEROOT_LOG, CHECKPOINT_FILE, NODE_ERROR_LOG,
  COMPARATOR_ERROR_LOG, NODE_LABEL, COMPARATOR_LABEL, FATAL_RE
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET_HEIGHT="$2"
      shift 2
      ;;
    --target=*)
      TARGET_HEIGHT="${1#*=}"
      shift
      ;;
    --interval)
      INTERVAL_SECONDS="$2"
      shift 2
      ;;
    --interval=*)
      INTERVAL_SECONDS="${1#*=}"
      shift
      ;;
    --local-rpc)
      LOCAL_RPC="$2"
      shift 2
      ;;
    --local-rpc=*)
      LOCAL_RPC="${1#*=}"
      shift
      ;;
    --once)
      ONCE=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

read_checkpoint() {
  local checkpoint
  if [[ ! -f "${CHECKPOINT_FILE}" ]]; then
    echo 0
    return
  fi

  checkpoint="$(tr -dc '0-9' < "${CHECKPOINT_FILE}" 2>/dev/null || true)"
  if [[ -n "${checkpoint}" ]]; then
    echo "${checkpoint}"
  else
    echo 0
  fi
}

read_node_height() {
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
    "${LOCAL_RPC}" |
    sed -n 's/.*"result":\([0-9][0-9]*\).*/\1/p'
}

service_has_pid() {
  local label="$1"
  local uid="${UID:-$(id -u)}"

  if launchctl print "gui/${uid}/${label}" 2>/dev/null | rg -q '^[[:space:]]*pid = [0-9]+'; then
    return 0
  fi

  (launchctl list "${label}" 2>/dev/null || true) | rg -q '"PID" = [0-9]+'
}

report_matching_failure() {
  local checkpoint="$1"
  local node="$2"

  echo "$(date '+%H:%M:%S') MISMATCH detected checkpoint=${checkpoint} node=${node:-unknown}"
  tail -n 8 "${STATEROOT_LOG}" 2>/dev/null || true
}

report_fatal_failure() {
  local checkpoint="$1"
  local node="$2"

  echo "$(date '+%H:%M:%S') fatal pattern detected checkpoint=${checkpoint} node=${node:-unknown}"
  rg -n "${FATAL_RE}" "${NODE_ERROR_LOG}" "${COMPARATOR_ERROR_LOG}" 2>/dev/null || true
}

check_once() {
  local checkpoint node last

  checkpoint="$(read_checkpoint)"
  node="$(read_node_height || true)"
  last="$(tail -n 1 "${STATEROOT_LOG}" 2>/dev/null || true)"

  if rg -q "MISMATCH" "${STATEROOT_LOG}" 2>/dev/null; then
    report_matching_failure "${checkpoint}" "${node}"
    return 2
  fi

  if rg -q "${FATAL_RE}" "${NODE_ERROR_LOG}" "${COMPARATOR_ERROR_LOG}" 2>/dev/null; then
    report_fatal_failure "${checkpoint}" "${node}"
    return 3
  fi

  if ! service_has_pid "${NODE_LABEL}"; then
    echo "$(date '+%H:%M:%S') node service not running checkpoint=${checkpoint}"
    launchctl list "${NODE_LABEL}" 2>/dev/null || true
    return 4
  fi

  if ! service_has_pid "${COMPARATOR_LABEL}"; then
    echo "$(date '+%H:%M:%S') comparator service not running checkpoint=${checkpoint}"
    launchctl list "${COMPARATOR_LABEL}" 2>/dev/null || true
    return 5
  fi

  echo "$(date '+%H:%M:%S') node=${node:-unknown} checkpoint=${checkpoint} last=${last}"

  if [[ "${TARGET_HEIGHT}" =~ ^[0-9]+$ ]] &&
     [[ "${TARGET_HEIGHT}" -gt 0 ]] &&
     [[ "${checkpoint}" -ge "${TARGET_HEIGHT}" ]]; then
    echo "TARGET_REACHED checkpoint=${checkpoint} target=${TARGET_HEIGHT}"
    return 10
  fi

  return 0
}

while true; do
  status=0
  check_once || status=$?

  case "${status}" in
    0)
      ;;
    10)
      exit 0
      ;;
    *)
      exit "${status}"
      ;;
  esac

  if [[ "${ONCE}" == "true" ]]; then
    exit 0
  fi

  sleep "${INTERVAL_SECONDS}"
done
