#!/bin/bash
set -e

PACKAGE="$1"
NETWORK="${2:-testnet}"

if [ -z "$PACKAGE" ]; then
    echo "Usage: $0 <package.nef> [network]"
    exit 1
fi

echo "Building deployment transaction for $NETWORK"
echo "Package: $PACKAGE"
