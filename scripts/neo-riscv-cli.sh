#!/bin/bash
set -e

CMD="$1"
shift

case "$CMD" in
    build)
        ./scripts/build-contract.sh "$@"
        ;;
    package)
        ./scripts/package-contract.sh "$@"
        ;;
    deploy)
        ./scripts/deploy-contract.sh "$@"
        ;;
    *)
        echo "Usage: $0 {build|package|deploy} [args]"
        exit 1
        ;;
esac
