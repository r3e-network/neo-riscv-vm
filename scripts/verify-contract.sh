#!/bin/bash
set -e

PACKAGE="$1"

if [ -z "$PACKAGE" ]; then
    echo "Usage: $0 <package.nef>"
    exit 1
fi

echo "Verifying contract package..."
./scripts/verify-binary.sh "$PACKAGE"
echo "✓ Verification complete"
