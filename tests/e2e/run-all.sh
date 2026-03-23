#!/bin/bash
set -e

echo "=== Running All E2E Tests ==="

# Run all test scripts
./tests/e2e/test-counter.sh
./tests/e2e/test-deploy-invoke.sh

echo ""
echo "✓ All E2E tests passed"
