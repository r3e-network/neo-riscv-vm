#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "${ROOT_DIR}/scripts/resolve-host-lib.sh"
PLUGIN_DIR="${ROOT_DIR}/dist/Plugins/Neo.Riscv.Adapter"

echo "Testing plugin loading package..."

dotnet test "${ROOT_DIR}/dotnet/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj"
"${ROOT_DIR}/scripts/package-adapter-plugin.sh"

test -f "${PLUGIN_DIR}/Neo.Riscv.Adapter.dll"

HOST_LIB_NAME="$(basename "$(resolve_host_lib "${ROOT_DIR}" release)")"
test -f "${PLUGIN_DIR}/${HOST_LIB_NAME}"

echo "✓ Plugin package built and validated"
