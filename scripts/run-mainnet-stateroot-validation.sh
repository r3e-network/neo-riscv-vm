#!/usr/bin/env bash
# =============================================================================
# Neo RISC-V VM — Mainnet State Root Validation
# =============================================================================
# Builds and launches neo-riscv-node with the RISC-V adapter and StateService
# plugin enabled. Monitors block sync and validates state roots against
# canonical mainnet values from seed node RPC.
#
# Usage:
#   ./scripts/run-mainnet-stateroot-validation.sh [--build-only] [--monitor-only]
#
# Prerequisites:
#   - Rust toolchain (for building libneo_riscv_host.so)
#   - .NET 10 SDK (for building neo-cli)
#   - neo-riscv-core and neo-riscv-node sibling repos
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORE_DIR="${CORE_DIR:-$HOME/git/neo-riscv-core}"
NODE_DIR="${NODE_DIR:-$HOME/git/neo-riscv-node}"

DEPLOY_DIR="${VM_DIR}/mainnet-validation"
DATA_DIR="${DEPLOY_DIR}/Data"
LOG_DIR="${DEPLOY_DIR}/logs"
STATEROOT_LOG="${LOG_DIR}/stateroot-validation.log"

REFERENCE_RPC="http://seed1.neo.org:10332"
LOCAL_RPC="http://127.0.0.1:10332"

BUILD_ONLY=false
MONITOR_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --build-only) BUILD_ONLY=true ;;
    --monitor-only) MONITOR_ONLY=true ;;
  esac
done

copy_plugin_output() {
  local name="$1"
  local output_dir="$2"

  if [[ ! -d "${output_dir}" ]]; then
    echo "  ERROR: plugin output directory not found: ${output_dir}" >&2
    exit 1
  fi

  mkdir -p "${DEPLOY_DIR}/Plugins/${name}"
  cp -a "${output_dir}/." "${DEPLOY_DIR}/Plugins/${name}/"
}

# ─── Build ───────────────────────────────────────────────────────────────────

build_all() {
  echo "=== Building RISC-V adapter plugin ==="
  bash "${VM_DIR}/scripts/package-adapter-plugin.sh"

  echo
  echo "=== Building neo-cli (Release) ==="
  (cd "${NODE_DIR}" && dotnet publish -o "${DEPLOY_DIR}" -c Release src/Neo.CLI)

  echo
  echo "=== Deploying plugins ==="
  mkdir -p "${DEPLOY_DIR}/Plugins"

  # Copy RISC-V adapter plugin
  cp -a "${VM_DIR}/dist/Plugins/." "${DEPLOY_DIR}/Plugins/"

  # Copy StateService plugin (built from node repo)
  echo "  Building StateService plugin..."
  (cd "${NODE_DIR}" && dotnet build -c Release plugins/StateService/StateService.csproj)
  copy_plugin_output "StateService" "${NODE_DIR}/plugins/StateService/bin/Release/net10.0"
  rm -f "${DEPLOY_DIR}/Plugins/StateService/RpcServer.dll" \
        "${DEPLOY_DIR}/Plugins/StateService/RpcServer.pdb" \
        "${DEPLOY_DIR}/Plugins/StateService/RpcServer.json"

  # Copy LevelDBStore plugin
  echo "  Building LevelDBStore plugin..."
  (cd "${NODE_DIR}" && dotnet build -c Release plugins/LevelDBStore/LevelDBStore.csproj)
  copy_plugin_output "LevelDBStore" "${NODE_DIR}/plugins/LevelDBStore/bin/Release/net10.0"

  # Copy RpcServer plugin (needed by StateService)
  echo "  Building RpcServer plugin..."
  (cd "${NODE_DIR}" && dotnet build -c Release plugins/RpcServer/RpcServer.csproj)
  copy_plugin_output "RpcServer" "${NODE_DIR}/plugins/RpcServer/bin/Release/net10.0"

  echo
  echo "=== Configuring for mainnet ==="

  # Use mainnet config
  cp "${NODE_DIR}/src/Neo.CLI/config.mainnet.json" "${DEPLOY_DIR}/config.json"

  # Configure StateService for full state root tracking
  mkdir -p "${DEPLOY_DIR}/Plugins/StateService"
  cat > "${DEPLOY_DIR}/Plugins/StateService/StateService.json" <<'STATECFG'
{
  "PluginConfiguration": {
    "Path": "Data_MPT_{0}",
    "FullState": true,
    "Network": 860833102,
    "AutoVerify": false,
    "MaxFindResultItems": 100,
    "UnhandledExceptionPolicy": "StopPlugin"
  },
  "Dependency": [
    "RpcServer"
  ]
}
STATECFG

  # Configure RpcServer to listen locally
  mkdir -p "${DEPLOY_DIR}/Plugins/RpcServer"
  cat > "${DEPLOY_DIR}/Plugins/RpcServer/RpcServer.json" <<'RPCCFG'
{
  "PluginConfiguration": {
    "Network": 860833102,
    "Servers": [
      {
        "BindAddress": "127.0.0.1",
        "Port": 10332,
        "SslCert": "",
        "SslCertPassword": "",
        "TrustedAuthorities": [],
        "RpcUser": "",
        "RpcPass": "",
        "MaxGasInvoke": 20,
        "MaxFee": 0.1,
        "MaxIteratorResultItems": 100,
        "MaxStackSize": 65535,
        "DisabledMethods": [],
        "SessionEnabled": false,
        "SessionExpirationTime": 60
      }
    ]
  }
}
RPCCFG

  mkdir -p "${DATA_DIR}" "${LOG_DIR}"

  echo
  echo "=== Build complete ==="
  echo "Deploy dir: ${DEPLOY_DIR}"
  echo "Plugins:"
  ls -la "${DEPLOY_DIR}/Plugins/" 2>/dev/null || true
}

# ─── Launch node ─────────────────────────────────────────────────────────────

launch_node() {
  echo "=== Launching neo-cli with RISC-V adapter (mainnet) ==="
  echo "Data:   ${DATA_DIR}"
  echo "Logs:   ${LOG_DIR}"
  echo "RPC:    ${LOCAL_RPC}"
  echo

  cd "${DEPLOY_DIR}"
  NEO_RISCV_HOST_LIB="${VM_DIR}/target/release/libneo_riscv_host.so" \
    dotnet Neo.CLI.dll \
    --noverify \
    2>&1 | tee "${LOG_DIR}/neo-cli.log" &

  NODE_PID=$!
  echo "${NODE_PID}" > "${DEPLOY_DIR}/neo-cli.pid"
  echo "Node PID: ${NODE_PID}"

  # Wait for RPC to come up
  echo "Waiting for local RPC..."
  for i in $(seq 1 60); do
    if timeout 2 curl -s -X POST -H 'Content-Type: application/json' \
       -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
       "${LOCAL_RPC}" >/dev/null 2>&1; then
      echo "Local RPC is up after ${i}s"
      return 0
    fi
    sleep 1
  done

  echo "WARNING: Local RPC not available after 60s, continuing anyway..."
}

# ─── State root monitor ─────────────────────────────────────────────────────

monitor_stateroots() {
  echo "=== State Root Validation Monitor ==="
  echo "Reference: ${REFERENCE_RPC}"
  echo "Local:     ${LOCAL_RPC}"
  echo "Log:       ${STATEROOT_LOG}"
  echo
  echo "Monitoring... (Ctrl+C to stop)"
  echo

  local last_checked=0
  local mismatches=0
  local checked=0

  # Header
  printf "%-10s %-68s %-8s\n" "Block" "StateRoot" "Status" | tee -a "${STATEROOT_LOG}"
  printf "%s\n" "$(printf '=%.0s' {1..90})" | tee -a "${STATEROOT_LOG}"

  while true; do
    # Get local block height
    local_height=$(timeout 5 curl -s -X POST -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' \
      "${LOCAL_RPC}" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',0))" 2>/dev/null || echo "0")

    if [[ "${local_height}" == "0" ]]; then
      echo "[$(date '+%H:%M:%S')] Waiting for node sync..."
      sleep 10
      continue
    fi

    local current_index=$((local_height - 1))

    # Check state root for each block we haven't checked yet
    local check_up_to=$current_index
    # Don't check the very latest block (might not have state root computed yet)
    check_up_to=$((check_up_to - 1))

    if [[ ${check_up_to} -le ${last_checked} ]]; then
      sleep 5
      continue
    fi

    for ((block = last_checked + 1; block <= check_up_to; block++)); do
      # Get local state root
      local_root=$(timeout 5 curl -s -X POST -H 'Content-Type: application/json' \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getstateroot\",\"params\":[${block}],\"id\":1}" \
        "${LOCAL_RPC}" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',{}).get('roothash','N/A'))" 2>/dev/null || echo "ERROR")

      # Get reference state root
      reference_root=$(timeout 5 curl -s -X POST -H 'Content-Type: application/json' \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getstateroot\",\"params\":[${block}],\"id\":1}" \
        "${REFERENCE_RPC}" 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('result',{}).get('roothash','N/A'))" 2>/dev/null || echo "ERROR")

      checked=$((checked + 1))

      if [[ "${local_root}" == "N/A" || "${local_root}" == "ERROR" ]]; then
        printf "%-10s %-68s %-8s\n" "${block}" "${local_root}" "SKIP" | tee -a "${STATEROOT_LOG}"
      elif [[ "${local_root}" == "${reference_root}" ]]; then
        # Only print every 100th block or first 10 to avoid noise
        if [[ $((block % 100)) -eq 0 || ${block} -le 10 ]]; then
          printf "%-10s %-68s %-8s\n" "${block}" "${local_root}" "OK" | tee -a "${STATEROOT_LOG}"
        fi
      else
        mismatches=$((mismatches + 1))
        printf "%-10s %-68s %-8s\n" "${block}" "LOCAL:${local_root}" "MISMATCH" | tee -a "${STATEROOT_LOG}"
        printf "%-10s %-68s\n" "" "REF:  ${reference_root}" | tee -a "${STATEROOT_LOG}"
        echo "!!! MISMATCH at block ${block} !!!" | tee -a "${STATEROOT_LOG}"
      fi

      last_checked=${block}
    done

    # Status update every pass
    echo "[$(date '+%H:%M:%S')] Checked: ${checked} blocks, Mismatches: ${mismatches}, Height: ${local_height}"

    sleep 2
  done
}

# ─── Main ────────────────────────────────────────────────────────────────────

main() {
  if [[ "${MONITOR_ONLY}" == "true" ]]; then
    monitor_stateroots
    exit 0
  fi

  build_all

  if [[ "${BUILD_ONLY}" == "true" ]]; then
    echo
    echo "Build complete. To launch:"
    echo "  cd ${DEPLOY_DIR} && NEO_RISCV_HOST_LIB=${VM_DIR}/target/release/libneo_riscv_host.so dotnet Neo.CLI.dll"
    echo
    echo "To monitor state roots:"
    echo "  $0 --monitor-only"
    exit 0
  fi

  launch_node
  monitor_stateroots
}

main "$@"
