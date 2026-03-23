#!/bin/bash
set -e

HASH="$1"
KEY="$2"

if [ -z "$HASH" ] || [ -z "$KEY" ]; then
    echo "Usage: $0 <contract_hash> <storage_key>"
    exit 1
fi

echo "Querying storage: $HASH[$KEY]"
