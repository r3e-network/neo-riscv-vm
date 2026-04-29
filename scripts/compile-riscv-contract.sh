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

CRATE_DIR="$(cd "${1:?Usage: compile-riscv-contract.sh <crate-dir> [output-path]}" && pwd)"
OUTPUT="${2:-${CRATE_DIR}/contract.polkavm}"

# Resolve the PolkaVM target JSON
if ! command -v polkatool &>/dev/null; then
    echo "Error: polkatool not found. Install with: cargo install polkavm-tools" >&2
    exit 1
fi

TARGET_DIR="${CRATE_DIR}/target"
TARGET_JSON="${TARGET_DIR}/neo-riscv32-polkavm.json"
TARGET="$(basename "${TARGET_JSON}" .json)"
ORIGINAL_TARGET_JSON="$(polkatool get-target-json-path -b 32)"
mkdir -p "${TARGET_DIR}"
if grep -q '"abi"' "${ORIGINAL_TARGET_JSON}"; then
    cp "${ORIGINAL_TARGET_JSON}" "${TARGET_JSON}"
else
    awk '
        /"llvm-abiname"[[:space:]]*:/ {
            print
            print "  \"abi\": \"ilp32e\","
            next
        }
        { print }
    ' "${ORIGINAL_TARGET_JSON}" > "${TARGET_JSON}"
fi

echo "Building contract crate: ${CRATE_DIR}"
cargo +nightly build \
    --manifest-path "${CRATE_DIR}/Cargo.toml" \
    --release \
    --target "${TARGET_JSON}" \
    --target-dir "${TARGET_DIR}" \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# Extract the package name from Cargo.toml. Cargo keeps hyphens in the final
# binary artifact name even though library artifacts use underscores.
PACKAGE_NAME=$(grep '^name' "${CRATE_DIR}/Cargo.toml" | head -1 | sed 's/.*= *"//' | sed 's/".*//')
UNDERSCORE_NAME=$(printf '%s' "${PACKAGE_NAME}" | tr '-' '_')

# The ELF lives under the target directory
ELF="${TARGET_DIR}/${TARGET}/release/${PACKAGE_NAME}"
if [ ! -f "${ELF}" ]; then
    ALT_ELF="${TARGET_DIR}/${TARGET}/release/${UNDERSCORE_NAME}"
    if [ -f "${ALT_ELF}" ]; then
        ELF="${ALT_ELF}"
    fi
fi

if [ ! -f "${ELF}" ]; then
    echo "Error: ELF not found at ${ELF}" >&2
    echo "Searching for it..." >&2
    find "${TARGET_DIR}" \( -name "${PACKAGE_NAME}" -o -name "${UNDERSCORE_NAME}" \) -type f 2>/dev/null || true
    exit 1
fi

echo "Linking PolkaVM blob..."
polkatool link --strip -o "${OUTPUT}" "${ELF}"
echo "Compiled: ${OUTPUT}"
