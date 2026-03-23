#!/bin/bash
set -e

BINARY="$1"
MANIFEST="$2"
OUTPUT="$3"

if [ -z "$BINARY" ] || [ -z "$MANIFEST" ] || [ -z "$OUTPUT" ]; then
    echo "Usage: $0 <binary.polkavm> <manifest.json> <output.nef>"
    exit 1
fi

# Create NEF package
cat "$BINARY" "$MANIFEST" > "$OUTPUT"
echo "✓ Package created: $OUTPUT"
