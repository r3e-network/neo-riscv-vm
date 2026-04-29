#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/dist/Plugins/Neo.Riscv.Adapter"

echo "[package] root: ${ROOT_DIR}"
echo "[package] out:  ${OUT_DIR}"

dotnet --version >/dev/null
cargo --version >/dev/null

mkdir -p "${OUT_DIR}"

echo "[package] building native host library (release)…"
cargo build -p neo-riscv-host --release

HOST_LIB_LINUX="${ROOT_DIR}/target/release/libneo_riscv_host.so"
HOST_LIB_MACOS="${ROOT_DIR}/target/release/libneo_riscv_host.dylib"
HOST_LIB_WINDOWS="${ROOT_DIR}/target/release/neo_riscv_host.dll"

HOST_LIB=""
if [[ "$(uname -s)" == "Linux" ]]; then
  HOST_LIB="${HOST_LIB_LINUX}"
elif [[ "$(uname -s)" == "Darwin" ]]; then
  HOST_LIB="${HOST_LIB_MACOS}"
else
  # MSYS/MinGW/Cygwin may report different values; fall back to checking files.
  if [[ -f "${HOST_LIB_WINDOWS}" ]]; then
    HOST_LIB="${HOST_LIB_WINDOWS}"
  fi
fi

if [[ -z "${HOST_LIB}" ]]; then
  if [[ -f "${HOST_LIB_LINUX}" ]]; then HOST_LIB="${HOST_LIB_LINUX}"; fi
  if [[ -z "${HOST_LIB}" && -f "${HOST_LIB_MACOS}" ]]; then HOST_LIB="${HOST_LIB_MACOS}"; fi
  if [[ -z "${HOST_LIB}" && -f "${HOST_LIB_WINDOWS}" ]]; then HOST_LIB="${HOST_LIB_WINDOWS}"; fi
fi

if [[ -z "${HOST_LIB}" ]]; then
  echo "[package] ERROR: native host library not found after build." >&2
  echo "[package] looked for:" >&2
  echo "  ${HOST_LIB_LINUX}" >&2
  echo "  ${HOST_LIB_MACOS}" >&2
  echo "  ${HOST_LIB_WINDOWS}" >&2
  exit 1
fi

echo "[package] building managed adapter plugin (Release)…"
dotnet build "${ROOT_DIR}/dotnet/Neo.Riscv.Adapter/Neo.Riscv.Adapter.csproj" -c Release

ADAPTER_DLL="${ROOT_DIR}/dotnet/Neo.Riscv.Adapter/bin/Release/net10.0/Neo.Riscv.Adapter.dll"
if [[ ! -f "${ADAPTER_DLL}" ]]; then
  echo "[package] ERROR: adapter DLL not found at ${ADAPTER_DLL}" >&2
  exit 1
fi

echo "[package] copying files…"
cp -f "${ADAPTER_DLL}" "${OUT_DIR}/Neo.Riscv.Adapter.dll"
cp -f "${HOST_LIB}" "${OUT_DIR}/$(basename "${HOST_LIB}")"

echo "[package] done."
echo "[package] install by copying '${ROOT_DIR}/dist/Plugins' next to your neo-cli binaries (same folder level as 'config.json')."
