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

echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_start target=${TARGET_HEIGHT} interval=${INTERVAL_SECONDS}"

last_status=0
services_stopped=false
latched=false

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
  elif [[ "${status}" != "${last_status}" ]]; then
    echo "$(date '+%Y-%m-%dT%H:%M:%S%z') watchdog_alert status=${status}"
  fi

  if [[ "${status}" != "0" ]]; then
    latch_after_failure "${status}"
  fi

  last_status="${status}"
  sleep "${INTERVAL_SECONDS}"
done
