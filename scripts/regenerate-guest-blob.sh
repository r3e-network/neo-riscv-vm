#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${ROOT_DIR}/target"
GUEST_MANIFEST="${ROOT_DIR}/crates/neo-riscv-guest-module/Cargo.toml"
GUEST_BLOB="${ROOT_DIR}/crates/neo-riscv-guest-module/guest.polkavm"
GUEST_TARGET_JSON="${TARGET_DIR}/neo-riscv32-polkavm.json"
GUEST_TARGET="$(basename "${GUEST_TARGET_JSON}" .json)"
GUEST_ELF="${TARGET_DIR}/${GUEST_TARGET}/release/neo-riscv-guest-module"
if [[ -n "${CARGO_NIGHTLY:-}" ]]; then
  read -r -a CARGO_NIGHTLY_CMD <<< "${CARGO_NIGHTLY}"
else
  CARGO_NIGHTLY_CMD=(cargo +nightly)
fi
if [[ -n "${RUSTC_NIGHTLY:-}" && -z "${RUSTC:-}" ]]; then
  export RUSTC="${RUSTC_NIGHTLY}"
fi

if ! command -v polkatool >/dev/null 2>&1; then
  echo "polkatool is required to regenerate guest.polkavm" >&2
  echo "Install it with: cargo install polkatool --version 0.32.0" >&2
  exit 1
fi

if ! "${CARGO_NIGHTLY_CMD[@]}" --version >/dev/null 2>&1; then
  echo "cargo +nightly is required to regenerate guest.polkavm" >&2
  echo "Set CARGO_NIGHTLY to a nightly cargo binary if rustup proxy is unavailable." >&2
  echo "Install it with: rustup toolchain install nightly" >&2
  exit 1
fi

mkdir -p "${TARGET_DIR}"
ORIGINAL_TARGET_JSON="$(polkatool get-target-json-path -b 32)"
if grep -q '"abi"' "${ORIGINAL_TARGET_JSON}"; then
  cp "${ORIGINAL_TARGET_JSON}" "${GUEST_TARGET_JSON}"
else
  awk '
    /"llvm-abiname"[[:space:]]*:/ {
      print
      print "  \"abi\": \"ilp32e\","
      next
    }
    { print }
  ' "${ORIGINAL_TARGET_JSON}" > "${GUEST_TARGET_JSON}"
fi

# Audit H14: the guest target has no unwinder, so the blob keeps abort
# semantics explicitly — the host profiles no longer carry panic="abort" for it.
# -Z location-detail=none strips absolute build paths (cargo/rustup/toolchain
# directories) that #[track_caller] panic locations otherwise embed into the
# blob's rodata: without it, blob bytes depend on the build machine's
# directories and OS, so a blob produced on one machine can never byte-match
# another machine's regeneration (proven red by the freshness gate's CI run).
export RUSTFLAGS="-C panic=abort -Z location-detail=none"

"${CARGO_NIGHTLY_CMD[@]}" build \
  --manifest-path "${GUEST_MANIFEST}" \
  --release \
  --target "${GUEST_TARGET_JSON}" \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec \
  --target-dir "${TARGET_DIR}"

polkatool link \
  --strip \
  -o "${GUEST_BLOB}" \
  "${GUEST_ELF}"

echo "Wrote ${GUEST_BLOB}"
