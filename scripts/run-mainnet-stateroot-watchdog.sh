#!/usr/bin/env bash
set -uo pipefail
export PATH="/Applications/Codex.app/Contents/Resources:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:${PATH:-}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

TARGET_HEIGHT="${1:-1500000}"
INTERVAL_SECONDS="${2:-60}"
NODE_LABEL="${NODE_LABEL:-com.codex.neo-riscv-node-mainnet}"
COMPARATOR_LABEL="${COMPARATOR_LABEL:-com.codex.neo-riscv-stateroot-mainnet}"
LAUNCHD_DOMAIN="${LAUNCHD_DOMAIN:-gui/$(id -u)}"
VALIDATION_DIR="${VALIDATION_DIR:-${VM_DIR}/mainnet-validation}"
LOG_DIR="${LOG_DIR:-${VALIDATION_DIR}/logs}"
CHECKPOINT_FILE="${CHECKPOINT_FILE:-${LOG_DIR}/stateroot-continuous.checkpoint}"
LOCAL_RPC="${LOCAL_RPC:-http://127.0.0.1:10332}"
PYTHON_BIN="${PYTHON_BIN:-/opt/homebrew/bin/python3}"
SNAPSHOT_ENABLED="${SNAPSHOT_ENABLED:-1}"
SNAPSHOT_ROOT="${SNAPSHOT_ROOT:-${VALIDATION_DIR}/resume-snapshots}"
SNAPSHOT_INTERVAL_BLOCKS="${SNAPSHOT_INTERVAL_BLOCKS:-100000}"
SNAPSHOT_KEEP="${SNAPSHOT_KEEP:-4}"
SNAPSHOT_PAUSE_SERVICES="${SNAPSHOT_PAUSE_SERVICES:-1}"
SNAPSHOT_COPY_ATTEMPTS="${SNAPSHOT_COPY_ATTEMPTS:-3}"
SNAPSHOT_RETRY_DELAY="${SNAPSHOT_RETRY_DELAY:-2}"
SNAPSHOT_RESTART_ATTEMPTS="${SNAPSHOT_RESTART_ATTEMPTS:-12}"
SNAPSHOT_RESTART_DELAY="${SNAPSHOT_RESTART_DELAY:-5}"
LAUNCH_AGENTS_DIR="${LAUNCH_AGENTS_DIR:-${HOME:-/Users/$(id -un)}/Library/LaunchAgents}"
NODE_PLIST="${NODE_PLIST:-${LAUNCH_AGENTS_DIR}/${NODE_LABEL}.plist}"
COMPARATOR_PLIST="${COMPARATOR_PLIST:-${LAUNCH_AGENTS_DIR}/${COMPARATOR_LABEL}.plist}"

echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_start target=${TARGET_HEIGHT} interval=${INTERVAL_SECONDS} snapshot_enabled=${SNAPSHOT_ENABLED} snapshot_interval=${SNAPSHOT_INTERVAL_BLOCKS} snapshot_pause_services=${SNAPSHOT_PAUSE_SERVICES}"

last_status=0
services_stopped=false
latched=false

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

snapshot_due() {
  local checkpoint="$1"
  local latest latest_height

  latest="$("${PYTHON_BIN}" "${SCRIPT_DIR}/mainnet-validation-snapshot.py" latest \
    --validation-dir "${VALIDATION_DIR}" \
    --snapshot-root "${SNAPSHOT_ROOT}" 2>/dev/null || true)"
  latest_height="$(printf '%s\n' "${latest}" | sed -n 's/^height=\([0-9][0-9]*\).*/\1/p')"

  if ! [[ "${latest_height}" =~ ^[0-9]+$ ]]; then
    return 0
  fi

  if (( checkpoint - latest_height >= SNAPSHOT_INTERVAL_BLOCKS )); then
    return 0
  fi

  echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot snapshot_too-recent height=${checkpoint} path=${latest#* path=}"
  return 1
}

stop_snapshot_services() {
  if [[ "${SNAPSHOT_PAUSE_SERVICES}" != "1" ]]; then
    return 0
  fi

  echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_pausing_services"
  launchctl bootout "${LAUNCHD_DOMAIN}/${COMPARATOR_LABEL}" 2>/dev/null || true
  launchctl bootout "${LAUNCHD_DOMAIN}/${NODE_LABEL}" 2>/dev/null || true
  sleep 2
}

service_loaded() {
  local label="$1"
  launchctl print "${LAUNCHD_DOMAIN}/${label}" >/dev/null 2>&1
}

bootstrap_service_until_loaded() {
  local label="$1"
  local plist="$2"
  local attempt

  for (( attempt = 1; attempt <= SNAPSHOT_RESTART_ATTEMPTS; attempt++ )); do
    if service_loaded "${label}"; then
      echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_service_ready label=${label} attempt=${attempt}"
      return 0
    fi

    launchctl bootstrap "${LAUNCHD_DOMAIN}" "${plist}" 2>/dev/null || true
    sleep "${SNAPSHOT_RESTART_DELAY}"
  done

  if service_loaded "${label}"; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_service_ready label=${label} attempt=${SNAPSHOT_RESTART_ATTEMPTS}"
    return 0
  fi

  echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_service_restart_failed label=${label} attempts=${SNAPSHOT_RESTART_ATTEMPTS}"
  return 1
}

start_snapshot_services() {
  if [[ "${SNAPSHOT_PAUSE_SERVICES}" != "1" ]]; then
    return 0
  fi

  echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_resuming_services"
  bootstrap_service_until_loaded "${NODE_LABEL}" "${NODE_PLIST}" || true
  bootstrap_service_until_loaded "${COMPARATOR_LABEL}" "${COMPARATOR_PLIST}" || true
}

maybe_save_snapshot() {
  if [[ "${SNAPSHOT_ENABLED}" != "1" ]]; then
    return 0
  fi

  local checkpoint node output status
  checkpoint="$(read_checkpoint)"
  if ! [[ "${checkpoint}" =~ ^[0-9]+$ ]] || [[ "${checkpoint}" -le 0 ]]; then
    return 0
  fi

  node="$(read_node_height || true)"
  if ! [[ "${node}" =~ ^[0-9]+$ ]]; then
    node="${checkpoint}"
  fi

  snapshot_due "${checkpoint}" || return 0
  stop_snapshot_services

  status=0
  output="$("${PYTHON_BIN}" "${SCRIPT_DIR}/mainnet-validation-snapshot.py" save \
    --validation-dir "${VALIDATION_DIR}" \
    --snapshot-root "${SNAPSHOT_ROOT}" \
    --height "${checkpoint}" \
    --node-height "${node}" \
    --min-interval "${SNAPSHOT_INTERVAL_BLOCKS}" \
    --keep "${SNAPSHOT_KEEP}" \
    --copy-attempts "${SNAPSHOT_COPY_ATTEMPTS}" \
    --retry-delay "${SNAPSHOT_RETRY_DELAY}" 2>&1)" || status=$?

  start_snapshot_services

  if [[ "${status}" == "0" ]]; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot ${output}"
  else
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_snapshot_failed status=${status} checkpoint=${checkpoint} node=${node} output=${output}"
  fi
}

latch_after_failure() {
  local status="$1"

  if [[ "${services_stopped}" != "true" ]]; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_stopping_services status=${status}"
    launchctl bootout "${LAUNCHD_DOMAIN}/${COMPARATOR_LABEL}" 2>/dev/null || true
    launchctl bootout "${LAUNCHD_DOMAIN}/${NODE_LABEL}" 2>/dev/null || true
    services_stopped=true
  fi

  echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_latched status=${status} reason=validation_failure"
  latched=true
}

while true; do
  if [[ "${latched}" == "true" ]]; then
    sleep "${INTERVAL_SECONDS}"
    continue
  fi

  status=0
  bash "${SCRIPT_DIR}/watch-mainnet-stateroot-validation.sh" \
    --target "${TARGET_HEIGHT}" \
    --interval "${INTERVAL_SECONDS}" \
    --once || status=$?

  if [[ "${status}" == "10" ]]; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_target_reached"
    exit 0
  fi

  if [[ "${status}" == "0" ]]; then
    services_stopped=false
    maybe_save_snapshot
  elif [[ "${status}" != "${last_status}" ]]; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_alert status=${status}"
  fi

  if [[ "${status}" != "0" ]]; then
    latch_after_failure "${status}"
  fi

  last_status="${status}"
  sleep "${INTERVAL_SECONDS}"
done
