#!/usr/bin/env bash
# Compile a Rust crate into a PolkaVM contract blob.
#
# Usage: compile-riscv-contract.sh <crate-dir> [output-path]
#
# Requires:
#   - Rust nightly toolchain
#   - polkatool (from polkavm-tools)
#
# Example:
#   ./scripts/compile-riscv-contract.sh path/to/my-contract
#   ./scripts/compile-riscv-contract.sh path/to/my-contract path/to/output.polkavm

set -euo pipefail

CRATE_DIR="${1:?Usage: compile-riscv-contract.sh <crate-dir> [output-path]}"
OUTPUT="${2:-${CRATE_DIR}/contract.polkavm}"

# Resolve the PolkaVM target JSON
if command -v polkatool &>/dev/null; then
    TARGET_JSON="$(polkatool get-target-json-path -b 32)"
else
    echo "Error: polkatool not found. Install with: cargo install polkavm-tools" >&2
    exit 1
fi

TARGET="riscv32emac-unknown-none-polkavm"

echo "Building contract crate: ${CRATE_DIR}"
cargo +nightly build \
    --manifest-path "${CRATE_DIR}/Cargo.toml" \
    --release \
    --target "${TARGET_JSON}" \
    -Zbuild-std=core,alloc

# Extract the crate name from Cargo.toml and convert to underscore form
CRATE_NAME=$(grep '^name' "${CRATE_DIR}/Cargo.toml" | head -1 | sed 's/.*= *"//' | sed 's/".*//' | tr '-' '_')

# The ELF lives under the target directory
ELF="target/${TARGET}/release/${CRATE_NAME}"

if [ ! -f "${ELF}" ]; then
    echo "Error: ELF not found at ${ELF}" >&2
    echo "Searching for it..." >&2
    find target -name "${CRATE_NAME}" -type f 2>/dev/null || true
    exit 1
fi

echo "Linking PolkaVM blob..."
polkatool link --strip -o "${OUTPUT}" "${ELF}"
echo "Compiled: ${OUTPUT}"
