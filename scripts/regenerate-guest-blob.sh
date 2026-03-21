#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${ROOT_DIR}/target"
GUEST_MANIFEST="${ROOT_DIR}/crates/neo-riscv-guest-module/Cargo.toml"
GUEST_TARGET="riscv32emac-unknown-none-polkavm"
GUEST_TARGET_JSON="$(polkatool get-target-json-path -b 32)"
GUEST_ELF="${TARGET_DIR}/${GUEST_TARGET}/release/neo-riscv-guest-module"
GUEST_BLOB="${ROOT_DIR}/crates/neo-riscv-guest-module/guest.polkavm"

if ! command -v polkatool >/dev/null 2>&1; then
  echo "polkatool is required to regenerate guest.polkavm" >&2
  echo "Install it with: cargo install polkatool --version 0.32.0" >&2
  exit 1
fi

if ! cargo +nightly --version >/dev/null 2>&1; then
  echo "cargo +nightly is required to regenerate guest.polkavm" >&2
  echo "Install it with: rustup toolchain install nightly" >&2
  exit 1
fi

cargo +nightly build \
  --manifest-path "${GUEST_MANIFEST}" \
  --release \
  --target "${GUEST_TARGET_JSON}" \
  -Zbuild-std=core,alloc \
  --target-dir "${TARGET_DIR}"

polkatool link \
  --strip \
  -o "${GUEST_BLOB}" \
  "${GUEST_ELF}"

echo "Wrote ${GUEST_BLOB}"
