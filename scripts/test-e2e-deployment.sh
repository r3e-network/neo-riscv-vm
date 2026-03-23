#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Testing contract packaging and deploy/invoke workflow..."
"${ROOT_DIR}/tests/e2e/run-all.sh"
echo "✓ E2E deployment workflow completed"
