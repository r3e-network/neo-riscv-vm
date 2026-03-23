#!/bin/bash
set -e

INPUT="$1"
OUTPUT="$2"

if [ -z "$INPUT" ] || [ -z "$OUTPUT" ]; then
    echo "Usage: $0 <input.elf> <output.polkavm>"
    exit 1
fi

polkatool link --format raw "$INPUT" -o "$OUTPUT"
