#!/bin/bash
# Check progress of all running mainnet validation processes
set -e

TOOL_DIR="/home/neo/git/neo-riscv-vm/tools"
MAINNET_HEIGHT=$(curl -s -X POST http://seed1.neo.org:10332 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}' 2>/dev/null | \
  python3 -c "import sys,json; print(json.load(sys.stdin)['result'])" 2>/dev/null || echo "unknown")

echo "══════════════════════════════════════════════════════════════"
echo "  Neo RISC-V VM — Mainnet Validation Progress"
echo "  $(date '+%Y-%m-%d %H:%M:%S')  │  Mainnet height: $MAINNET_HEIGHT"
echo "══════════════════════════════════════════════════════════════"
echo ""

# Helper: extract last block from log (match lines starting with digits followed by tab)
last_block() {
  local log="$1"
  if [ -f "$log" ]; then
    grep -P '^\d+\t' "$log" | tail -1 | cut -f1
  fi
}

last_match() {
  local log="$1"
  if [ -f "$log" ]; then
    grep -P '^\d+\t' "$log" | tail -1
  fi
}

# 1. Empty-block RISC-V validator
echo "── Empty-block RISC-V (native contract parity) ──"
if pgrep -f "StateRootValidator.*riscv-only" > /dev/null 2>&1; then
  echo "  Status: RUNNING"
else
  echo "  Status: stopped"
fi
RISCV_BLOCK=$(last_block "$TOOL_DIR/StateRootValidator/riscv-v4.log")
echo "  Last block: ${RISCV_BLOCK:-N/A}"
echo "  Last entry: $(last_match "$TOOL_DIR/StateRootValidator/riscv-v4.log")"
echo ""

# 2. Empty-block NeoVM validator
echo "── Empty-block NeoVM (baseline) ──"
if pgrep -f "StateRootValidator.*neovm" > /dev/null 2>&1; then
  echo "  Status: RUNNING"
else
  echo "  Status: stopped"
fi
NEOVM_BLOCK=$(last_block "$TOOL_DIR/StateRootValidator/neovm-v4.log")
echo "  Last block: ${NEOVM_BLOCK:-N/A}"
echo "  Last entry: $(last_match "$TOOL_DIR/StateRootValidator/neovm-v4.log")"
echo ""

# 3. Full-block validator (real transactions)
echo "── Full-block Validator (real transaction replay) ──"
if pgrep -f "FullBlockValidator" > /dev/null 2>&1; then
  echo "  Status: RUNNING"
else
  echo "  Status: stopped"
fi
FULL_BLOCK=$(last_block "$TOOL_DIR/FullBlockValidator/fullblock-run.log")
echo "  Last block: ${FULL_BLOCK:-N/A}"
echo "  Last entry: $(last_match "$TOOL_DIR/FullBlockValidator/fullblock-run.log")"
if [ -f "$TOOL_DIR/FullBlockValidator/fullblock-run.log" ]; then
  MISMATCHES=$(grep -c "MISMATCH" "$TOOL_DIR/FullBlockValidator/fullblock-run.log" 2>/dev/null || echo "0")
  FAULTS=$(grep -c "tx-fault" "$TOOL_DIR/FullBlockValidator/fullblock-run.log" 2>/dev/null || echo "0")
  echo "  Mismatches: $MISMATCHES"
  echo "  Tx faults logged: $FAULTS"
fi
echo ""

# 4. Fingerprint comparison for empty-block validators
echo "── Empty-block fingerprint comparison (overlapping checkpoints) ──"
RISCV_CP="$TOOL_DIR/StateRootValidator/riscv-v4-checkpoint.txt"
NEOVM_CP="$TOOL_DIR/StateRootValidator/neovm-v4-checkpoint.txt"
if [ -f "$RISCV_CP" ] && [ -f "$NEOVM_CP" ]; then
  mismatches=0
  matches=0
  while IFS=$'\t' read -r block fp entries; do
    neovm_fp=$(grep "^$block	" "$NEOVM_CP" 2>/dev/null | cut -f2)
    if [ -n "$neovm_fp" ]; then
      if [ "$fp" = "$neovm_fp" ]; then
        matches=$((matches + 1))
      else
        mismatches=$((mismatches + 1))
        echo "  MISMATCH at block $block: RISC-V=$fp NeoVM=$neovm_fp"
      fi
    fi
  done < "$RISCV_CP"
  echo "  Overlapping checkpoints compared: $matches matches, $mismatches mismatches"
else
  echo "  Checkpoint files not yet available"
fi
echo ""

# Summary
echo "══════════════════════════════════════════════════════════════"
TOTAL_VALIDATED=0
[ -n "$RISCV_BLOCK" ] && [ "$RISCV_BLOCK" -gt "$TOTAL_VALIDATED" ] 2>/dev/null && TOTAL_VALIDATED=$RISCV_BLOCK
echo "  Empty-block progress: ${TOTAL_VALIDATED:-0} / $MAINNET_HEIGHT blocks"
echo "  Full-block progress:  ${FULL_BLOCK:-0} / $MAINNET_HEIGHT blocks"
echo "══════════════════════════════════════════════════════════════"
