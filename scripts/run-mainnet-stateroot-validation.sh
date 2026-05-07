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
CORE_DIR="${CORE_DIR:-$(cd "${VM_DIR}/../neo-riscv-core" 2>/dev/null && pwd)}"
NODE_DIR="${NODE_DIR:-$(cd "${VM_DIR}/../neo-riscv-node" 2>/dev/null && pwd)}"
. "${SCRIPT_DIR}/resolve-host-lib.sh"
HOST_LIB="${HOST_LIB:-$(resolve_host_lib "${VM_DIR}" release)}"

DEPLOY_DIR="${VM_DIR}/mainnet-validation"
DATA_DIR="${DEPLOY_DIR}/Data"
LOG_DIR="${DEPLOY_DIR}/logs"
STATEROOT_LOG="${LOG_DIR}/stateroot-validation.log"
STATEROOT_CHECKPOINT="${LOG_DIR}/stateroot-validation.checkpoint"

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

stage_leveldb_native() {
  local rid
  case "$(uname -s):$(uname -m)" in
    Darwin:arm64) rid="osx-arm64" ;;
    Darwin:x86_64) rid="osx-x64" ;;
    Linux:aarch64|Linux:arm64) rid="linux-arm64" ;;
    Linux:x86_64) rid="linux-x64" ;;
    *) return 0 ;;
  esac

  local native_dir="${DEPLOY_DIR}/Plugins/LevelDBStore/runtimes/${rid}/native"
  local native_lib
  case "${rid}" in
    osx-*) native_lib="${native_dir}/libleveldb.dylib" ;;
    linux-*) native_lib="${native_dir}/libleveldb.so" ;;
    *) return 0 ;;
  esac

  if [[ -f "${native_lib}" ]]; then
    cp "${native_lib}" "${DEPLOY_DIR}/Plugins/LevelDBStore/"
    cp "${native_lib}" "${DEPLOY_DIR}/"
  fi
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
  stage_leveldb_native

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
    "UnhandledExceptionPolicy": "Ignore",
    "Servers": [
      {
        "Network": 860833102,
        "BindAddress": "127.0.0.1",
        "Port": 10332,
        "SslCert": "",
        "SslCertPassword": "",
        "TrustedAuthorities": [],
        "RpcUser": "",
        "RpcPass": "",
        "EnableCors": true,
        "AllowOrigins": [],
        "KeepAliveTimeout": 60,
        "RequestHeadersTimeout": 15,
        "MaxGasInvoke": 20,
        "MaxFee": 0.1,
        "MaxConcurrentConnections": 40,
        "MaxIteratorResultItems": 100,
        "MaxStackSize": 65535,
        "DisabledMethods": [ "openwallet" ],
        "SessionEnabled": false,
        "SessionExpirationTime": 60,
        "FindStoragePageSize": 50
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
  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
    dotnet neo-cli.dll \
    --noverify \
    --background \
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
  echo "Checkpoint:${STATEROOT_CHECKPOINT}"
  echo
  echo "Monitoring with JSON-RPC batches... (Ctrl+C to stop)"
  echo

  mkdir -p "${LOG_DIR}"

  local start_index="${STATEROOT_START:-0}"
  if [[ -f "${STATEROOT_CHECKPOINT}" ]]; then
    local checkpoint
    checkpoint="$(tr -dc '0-9' < "${STATEROOT_CHECKPOINT}" || true)"
    if [[ -n "${checkpoint}" ]]; then
      start_index=$((checkpoint + 1))
    fi
  fi

  python3 "${SCRIPT_DIR}/compare-stateroot-rpc-batch.py" \
    --local-rpc "${LOCAL_RPC}" \
    --reference-rpc "${REFERENCE_RPC}" \
    --start "${start_index}" \
    --follow \
    --log-file "${STATEROOT_LOG}" \
    --checkpoint-file "${STATEROOT_CHECKPOINT}"
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
    echo "  cd ${DEPLOY_DIR} && NEO_RISCV_HOST_LIB=${HOST_LIB} dotnet neo-cli.dll --noverify --background"
    echo
    echo "To monitor state roots:"
    echo "  $0 --monitor-only"
    exit 0
  fi

  launch_node
  monitor_stateroots
}

main "$@"
