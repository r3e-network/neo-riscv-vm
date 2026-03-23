#!/bin/bash
set -e

BINARY="$1"

if [ -z "$BINARY" ]; then
    echo "Usage: $0 <contract.polkavm>"
    exit 1
fi

# Check magic bytes
head -c 4 "$BINARY" | xxd -p | grep -q "50564d00" || {
    echo "Error: Invalid PolkaVM magic"
    exit 1
}

echo "✓ Binary format valid"
