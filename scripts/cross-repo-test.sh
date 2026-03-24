#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CORE_DIR="${CORE_DIR:-$HOME/git/neo-riscv-core}"
NODE_DIR="${NODE_DIR:-$HOME/git/neo-riscv-node}"

HOST_LIB="${VM_DIR}/target/release/libneo_riscv_host.so"
PLUGIN_BUNDLE_DIR="${VM_DIR}/dist/Plugins"

CORE_PROJECTS=(
  "${CORE_DIR}/tests/Neo.Extensions.Tests/Neo.Extensions.Tests.csproj"
  "${CORE_DIR}/tests/Neo.Json.UnitTests/Neo.Json.UnitTests.csproj"
  "${CORE_DIR}/tests/Neo.UnitTests/Neo.UnitTests.csproj"
)

NODE_PROJECTS=(
  "${NODE_DIR}/tests/Neo.CLI.Tests/Neo.CLI.Tests.csproj"
  "${NODE_DIR}/tests/Neo.ConsoleService.Tests/Neo.ConsoleService.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Cryptography.MPTTrie.Tests/Neo.Cryptography.MPTTrie.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Network.RPC.Tests/Neo.Network.RPC.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.ApplicationLogs.Tests/Neo.Plugins.ApplicationLogs.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.DBFTPlugin.Tests/Neo.Plugins.DBFTPlugin.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.OracleService.Tests/Neo.Plugins.OracleService.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.RestServer.Tests/Neo.Plugins.RestServer.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.RpcServer.Tests/Neo.Plugins.RpcServer.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.SQLiteWallet.Tests/Neo.Plugins.SQLiteWallet.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.SignClient.Tests/Neo.Plugins.SignClient.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.StateService.Tests/Neo.Plugins.StateService.Tests.csproj"
  "${NODE_DIR}/tests/Neo.Plugins.Storage.Tests/Neo.Plugins.Storage.Tests.csproj"
)

require_dir() {
  local path="$1"
  if [[ ! -d "${path}" ]]; then
    echo "Missing directory: ${path}" >&2
    exit 1
  fi
}

run_step() {
  local title="$1"
  shift
  echo
  echo "=== ${title} ==="
  "$@"
}

stage_plugin_for_project() {
  local project="$1"
  local output_dir
  output_dir="$(dirname "${project}")/bin/Debug/net10.0/Plugins"

  rm -rf "${output_dir}"

  if [[ "$(basename "${project}")" == "Neo.CLI.Tests.csproj" ]]; then
    mkdir -p "${output_dir}"
    cp -a "${PLUGIN_BUNDLE_DIR}/." "${output_dir}/"
  fi
}

run_core_matrix() {
  local project
  for project in "${CORE_PROJECTS[@]}"; do
    if [[ ! -f "${project}" ]]; then
      echo "Missing core test project: ${project}" >&2
      exit 1
    fi

    if [[ "$(basename "${project}")" == "Neo.UnitTests.csproj" ]]; then
      mkdir -p "$(dirname "${project}")/bin/Debug/net10.0/Plugins"
      cp -a "${PLUGIN_BUNDLE_DIR}/." "$(dirname "${project}")/bin/Debug/net10.0/Plugins/"
    fi

    echo
    echo "RUN $(basename "${project}")"
    NEO_RISCV_HOST_LIB="${HOST_LIB}" dotnet test "${project}" -m:1 --logger "console;verbosity=minimal"
    echo "PASS $(basename "${project}")"
  done
}

run_node_matrix() {
  local project
  for project in "${NODE_PROJECTS[@]}"; do
    if [[ ! -f "${project}" ]]; then
      echo "Missing node test project: ${project}" >&2
      exit 1
    fi

    stage_plugin_for_project "${project}"

    echo
    echo "RUN $(basename "${project}")"
    NEO_RISCV_HOST_LIB="${HOST_LIB}" dotnet test "${project}" -m:1 --logger "console;verbosity=minimal"
    echo "PASS $(basename "${project}")"
  done
}

run_node_cli_smoke() {
  echo
  echo "RUN neo-cli smoke"
  (
    cd "${NODE_DIR}"
    dotnet publish -o ./out -c Release src/Neo.CLI
    find ./out -name 'config.json' | xargs perl -pi -e 's|LevelDBStore|MemoryStore|g'
    mkdir -p ./out/Plugins
    cp -a "${PLUGIN_BUNDLE_DIR}/." ./out/Plugins/
    find . -maxdepth 1 -name 'test-wallet*.json' -delete
    expect ./.github/workflows/test-neo-cli.expect
  )
  echo "PASS neo-cli smoke"
}

main() {
  require_dir "${VM_DIR}"
  require_dir "${CORE_DIR}"
  require_dir "${NODE_DIR}"

  echo "=== Cross-Repo Validation ==="
  echo "VM:   ${VM_DIR}"
  echo "Core: ${CORE_DIR}"
  echo "Node: ${NODE_DIR}"

  run_step "Package Adapter Plugin" "${VM_DIR}/scripts/package-adapter-plugin.sh"

  if [[ ! -f "${HOST_LIB}" ]]; then
    echo "Missing release host library: ${HOST_LIB}" >&2
    exit 1
  fi

  run_step "VM Verification" "${VM_DIR}/scripts/verify-all.sh"
  run_step "VM E2E" "${VM_DIR}/tests/e2e/run-all.sh"
  run_step "VM FFI Resolution" "${VM_DIR}/scripts/test-ffi-resolution.sh"
  run_step "Core Test Matrix" run_core_matrix
  run_step "Node Test Matrix" run_node_matrix
  run_step "Node CLI Smoke" run_node_cli_smoke

  echo
  echo "✓ Cross-repo validation complete"
}

main "$@"
