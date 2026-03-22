#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST_LIB="${ROOT_DIR}/target/release/libneo_riscv_host.so"
PREFERRED_NEO_TEST_PROJECT="${ROOT_DIR}/../neo-riscv-core/tests/Neo.UnitTests/Neo.UnitTests.csproj"
FALLBACK_NEO_TEST_PROJECT="/home/neo/.config/superpowers/worktrees/neo/master-n3-riscv-interpreter/tests/Neo.UnitTests/Neo.UnitTests.csproj"
NEO_TEST_PROJECT="${NEO_TEST_PROJECT:-}"
NEO_TEST_PROJECT_EXPLICIT="0"
if [[ -n "${NEO_TEST_PROJECT}" ]]; then
  NEO_TEST_PROJECT_EXPLICIT="1"
fi
NEO_TEST_FILTER="${NEO_TEST_FILTER:-}"
NEO_RUN_NEO_UNITTESTS="${NEO_RUN_NEO_UNITTESTS:-}"
NEO_TEST_CONFIGURATION="${NEO_TEST_CONFIGURATION:-Debug}"

cargo test -p neo-riscv-guest -p neo-riscv-host
cargo build -p neo-riscv-host --release

NEO_RISCV_HOST_LIB="${HOST_LIB}" \
NEO_RISCV_VM_JSON_MODE=full \
dotnet test "${ROOT_DIR}/compat/Neo.VM.Riscv.Tests/Neo.VM.Riscv.Tests.csproj"

dotnet test "${ROOT_DIR}/compat/Neo.Riscv.Adapter.Tests/Neo.Riscv.Adapter.Tests.csproj"

if [[ -z "${NEO_TEST_PROJECT}" ]]; then
  if [[ -f "${PREFERRED_NEO_TEST_PROJECT}" ]]; then
    NEO_TEST_PROJECT="${PREFERRED_NEO_TEST_PROJECT}"
  elif [[ -f "${FALLBACK_NEO_TEST_PROJECT}" ]]; then
    NEO_TEST_PROJECT="${FALLBACK_NEO_TEST_PROJECT}"
  fi
fi

if [[ "${NEO_RUN_NEO_UNITTESTS}" == "1" || -n "${NEO_TEST_FILTER}" || "${NEO_TEST_PROJECT_EXPLICIT}" == "1" ]]; then
  if [[ -z "${NEO_TEST_PROJECT}" ]]; then
    echo "[verify-all] Skipping Neo.UnitTests: set NEO_TEST_PROJECT to a Neo.UnitTests.csproj path." >&2
    exit 0
  fi

  if [[ ! -f "${NEO_TEST_PROJECT}" ]]; then
    echo "[verify-all] Skipping Neo.UnitTests: project not found: ${NEO_TEST_PROJECT}" >&2
    exit 0
  fi

  DOTNET_TEST_ARGS=("${NEO_TEST_PROJECT}")
  if [[ -n "${NEO_TEST_CONFIGURATION}" ]]; then
    DOTNET_TEST_ARGS+=("-c" "${NEO_TEST_CONFIGURATION}")
  fi
  if [[ -n "${NEO_TEST_FILTER}" ]]; then
    DOTNET_TEST_ARGS+=("--filter" "${NEO_TEST_FILTER}")
  fi

  NEO_RISCV_HOST_LIB="${HOST_LIB}" \
  dotnet test "${DOTNET_TEST_ARGS[@]}"
else
  echo "[verify-all] Skipping Neo.UnitTests (can be slow). To run:" >&2
  echo "  NEO_RUN_NEO_UNITTESTS=1 scripts/verify-all.sh" >&2
  echo "  NEO_TEST_FILTER=FullyQualifiedName~SomeTest scripts/verify-all.sh" >&2
fi
