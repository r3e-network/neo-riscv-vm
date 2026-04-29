#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLUGIN_DIR="${ROOT_DIR}/dist/Plugins/Neo.Riscv.Adapter"

echo "Testing plugin loading package..."

dotnet test "${ROOT_DIR}/dotnet/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj"
"${ROOT_DIR}/scripts/package-adapter-plugin.sh"

test -f "${PLUGIN_DIR}/Neo.Riscv.Adapter.dll"

if [[ "$(uname -s)" == "Linux" ]]; then
  test -f "${PLUGIN_DIR}/libneo_riscv_host.so"
fi

echo "✓ Plugin package built and validated"
