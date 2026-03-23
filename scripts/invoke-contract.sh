#!/bin/bash
set -e

HASH="$1"
METHOD="$2"
shift 2
ARGS="$@"

if [ -z "$HASH" ] || [ -z "$METHOD" ]; then
    echo "Usage: $0 <contract_hash> <method> [args...]"
    exit 1
fi

echo "Invoking: $HASH.$METHOD($ARGS)"
