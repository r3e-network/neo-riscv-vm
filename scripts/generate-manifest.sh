#!/bin/bash
set -e

NAME="$1"
OUTPUT="${2:-manifest.json}"

cat > "$OUTPUT" <<EOF
{
  "name": "$NAME",
  "groups": [],
  "features": {},
  "supportedstandards": [],
  "abi": {
    "methods": [],
    "events": []
  },
  "permissions": [],
  "trusts": [],
  "extra": null
}
EOF

echo "Generated: $OUTPUT"
