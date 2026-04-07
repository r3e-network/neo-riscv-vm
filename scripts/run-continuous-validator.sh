#!/usr/bin/env bash
# =============================================================================
# Continuous FullBlockValidator runner with auto-restart
# =============================================================================
# Resumes from checkpoint, auto-restarts on crash, logs to timestamped files.
# Usage: ./scripts/run-continuous-validator.sh [--start N]
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
VALIDATOR_DIR="$PROJECT_DIR/tools/FullBlockValidator"
NATIVE_LIB="$PROJECT_DIR/target/release/libneo_riscv_host.so"
LOG_DIR="$VALIDATOR_DIR/logs"

mkdir -p "$LOG_DIR"

# Build native library if needed
if [ ! -f "$NATIVE_LIB" ]; then
    echo "Building native library..."
    cd "$PROJECT_DIR"
    cargo build --release 2>&1 | tail -5
fi

# Build validator if needed
if [ ! -f "$VALIDATOR_DIR/bin/Release/net10.0/FullBlockValidator.dll" ]; then
    echo "Building FullBlockValidator..."
    cd "$VALIDATOR_DIR"
    dotnet build -c Release 2>&1 | tail -5
fi

# Copy native lib to validator output directory
mkdir -p "$VALIDATOR_DIR/bin/Release/net10.0/Plugins/Neo.Riscv.Adapter/"
cp "$NATIVE_LIB" "$VALIDATOR_DIR/bin/Release/net10.0/Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so" 2>/dev/null || true

# Ensure adapter DLL is in place
if [ ! -f "$VALIDATOR_DIR/bin/Release/net10.0/Neo.Riscv.Adapter.dll" ]; then
    cp "$PROJECT_DIR/compat/Neo.Riscv.Adapter/obj/Release/net10.0/Neo.Riscv.Adapter.dll" \
       "$VALIDATOR_DIR/bin/Release/net10.0/Neo.Riscv.Adapter.dll" 2>/dev/null || true
fi

export NEO_RISCV_HOST_LIB="$NATIVE_LIB"

# Parse --start argument
START_ARG=""
for arg in "$@"; do
    case $arg in
        --start=*) START_ARG="--start ${arg#--start=}" ;;
        --start) ;;
        [0-9]*) START_ARG="--start $arg" ;;
    esac
done

# Auto-restart loop
RUN=0
CONSECUTIVE_CRASHES=0
MAX_CONSECUTIVE=5

while true; do
    RUN=$((RUN + 1))
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    LOGFILE="$LOG_DIR/validator-run-${RUN}-${TIMESTAMP}.log"

    echo "=== Run #$RUN starting at $(date) ==="
    echo "=== Log: $LOGFILE ==="

    cd "$VALIDATOR_DIR"

    dotnet run -c Release -- \
        --rpc http://seed1.neo.org:10332 \
        --state-dir "$LOG_DIR/state" \
        --checkpoint "$LOG_DIR/fullblock-checkpoint.txt" \
        --batch-size 50 \
        --save-interval 5000 \
        --fp-interval 1000 \
        --report-interval 100 \
        $START_ARG \
        2>&1 | tee "$LOGFILE"

    EXIT_CODE=${PIPESTATUS[0]}

    if [ $EXIT_CODE -eq 0 ]; then
        echo "=== Validation completed successfully at $(date) ==="
        CONSECUTIVE_CRASHES=0
        break
    fi

    CONSECUTIVE_CRASHES=$((CONSECUTIVE_CRASHES + 1))
    echo "=== Run #$RUN exited with code $EXIT_CODE (consecutive crashes: $CONSECUTIVE_CRASHES/$MAX_CONSECUTIVE) ==="

    if [ $CONSECUTIVE_CRASHES -ge $MAX_CONSECUTIVE ]; then
        echo "=== Too many consecutive crashes. Stopping. ==="
        exit 1
    fi

    # Read checkpoint for resume
    if [ -f "$LOG_DIR/fullblock-checkpoint.txt" ]; then
        LAST_LINE=$(tail -1 "$LOG_DIR/fullblock-checkpoint.txt")
        LAST_BLOCK=$(echo "$LAST_LINE" | cut -f1)
        START_ARG="--start $((LAST_BLOCK + 1))"
        echo "=== Resuming from block $((LAST_BLOCK + 1)) ==="
    fi

    echo "=== Restarting in 10 seconds... ==="
    sleep 10
done
