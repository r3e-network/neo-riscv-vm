#!/usr/bin/env bash
# 24h soak check-in for FullBlockValidator started 2026-04-27.
# Read-only inspection. Writes a report to mainnet-validation/24h-checkin-report.txt.

set -u

VM_DIR="/home/neo/git/neo-riscv/neo-riscv-vm"
LOG="$VM_DIR/mainnet-validation/fullblock-validation-2026-04-27.log"
PID_FILE="$VM_DIR/mainnet-validation/fullblock-validator.pid"
REPORT="$VM_DIR/mainnet-validation/24h-checkin-report.txt"
CKPT="$VM_DIR/fullblock-checkpoint.txt"
HEARTBEAT="$VM_DIR/fullblock-current-block.txt"

START_BLOCK=770000
NOW="$(date)"

{
  echo "================================================================"
  echo "FullBlockValidator 24h check-in — generated $NOW"
  echo "================================================================"
  echo

  echo "## 1. Process state"
  if [ -f "$PID_FILE" ]; then
    PID="$(cat "$PID_FILE")"
    if ps -p "$PID" >/dev/null 2>&1; then
      ps -o pid,etime,rss,pcpu,cmd -p "$PID" 2>&1 | head -3
      echo "STATUS: RUNNING"
    else
      echo "PID $PID in pid-file but process is GONE."
      echo "Last 30 log lines (likely contains exit reason):"
      tail -30 "$LOG" 2>/dev/null | sed 's/^/  /'
      echo "STATUS: STOPPED"
    fi
  else
    echo "No pid-file at $PID_FILE"
    echo "STATUS: UNKNOWN"
  fi
  echo

  echo "## 2. Block progress"
  echo "Last 3 checkpoint lines:"
  tail -3 "$CKPT" 2>/dev/null | sed 's/^/  /'
  LAST_CKPT_BLOCK="$(tail -1 "$CKPT" 2>/dev/null | awk '{print $1}')"
  if [ -n "${LAST_CKPT_BLOCK:-}" ]; then
    PROCESSED=$((LAST_CKPT_BLOCK - START_BLOCK))
    echo "Blocks processed since $START_BLOCK: $PROCESSED"
    echo "Per-hour rate (assuming 24h soak): ~$((PROCESSED / 24))"
  fi
  echo
  echo "Heartbeat:"
  if [ -f "$HEARTBEAT" ]; then
    cat "$HEARTBEAT" 2>/dev/null | sed 's/^/  /'
    echo "  (file mtime: $(stat -c '%y' "$HEARTBEAT" 2>/dev/null))"
  else
    echo "  (no heartbeat file)"
  fi
  echo
  echo "Latest log tail (last 8 lines):"
  tail -8 "$LOG" 2>/dev/null | sed 's/^/  /'
  echo

  echo "## 3. Mismatch / divergence"
  MISMATCH_COUNT="$(grep -cE 'MISMATCH|DIVERGE' "$LOG" 2>/dev/null || true)"
  MISMATCH_COUNT="${MISMATCH_COUNT:-0}"
  echo "MISMATCH/DIVERGE line count: $MISMATCH_COUNT"
  if [ "${MISMATCH_COUNT:-0}" -gt 0 ]; then
    echo "*** HIGH SEVERITY *** sample lines:"
    grep -nE 'MISMATCH|DIVERGE' "$LOG" 2>/dev/null | head -10 | sed 's/^/  /'
  fi
  echo

  echo "## 4. Hang signal"
  # Note: fullblock-current-block.txt is not updated by the current validator binary,
  # so use the log file's mtime instead — if the log hasn't been written to in 5+ min
  # while the process is alive, that's a hang signal.
  GAP=0
  if [ -f "$LOG" ]; then
    LOG_EPOCH="$(stat -c '%Y' "$LOG" 2>/dev/null || echo 0)"
    NOW_EPOCH="$(date +%s)"
    GAP=$((NOW_EPOCH - LOG_EPOCH))
    echo "Log file last write: ${GAP}s ago"
    if [ "$GAP" -gt 300 ] && ps -p "${PID:-0}" >/dev/null 2>&1; then
      echo "*** WARN *** log >5min stale while process is alive — possible hang."
    fi
  fi
  echo

  echo "## 5. Fault-trace summary"
  TXFAULT_COUNT="$(grep -cE 'tx-fault-trace|tx-fault\]' "$LOG" 2>/dev/null || true)"
  TXFAULT_COUNT="${TXFAULT_COUNT:-0}"
  echo "Total tx-fault lines: $TXFAULT_COUNT"
  echo "Distinct fault messages (top 10):"
  grep 'tx-fault\]' "$LOG" 2>/dev/null \
    | sed -E 's/.*tx-fault\] block [0-9]+ tx 0x[0-9a-f]+: //' \
    | sort -u | head -10 | sed 's/^/  /'
  echo
  echo "Guest-side concerns (panic/unsupported/trap):"
  grep -nE 'panic|unsupported opcode|guest-trap|Operation is not valid|InvalidOperationException' "$LOG" 2>/dev/null \
    | head -10 | sed 's/^/  /' || echo "  (none)"
  echo

  echo "## 6. Verdict"
  VERDICT="GREEN"
  REASON="advancing, no mismatches, no hangs"
  if [ "${MISMATCH_COUNT:-0}" -gt 0 ]; then
    VERDICT="RED"
    REASON="$MISMATCH_COUNT MISMATCH/DIVERGE lines"
  elif ! ps -p "${PID:-0}" >/dev/null 2>&1; then
    VERDICT="RED"
    REASON="process not running"
  elif [ "${GAP:-0}" -gt 300 ]; then
    VERDICT="YELLOW"
    REASON="possible hang — log ${GAP}s stale"
  fi
  echo "$VERDICT — $REASON"
  echo
  echo "Full log: $LOG"
  echo "Report path: $REPORT"
} > "$REPORT" 2>&1

if command -v notify-send >/dev/null 2>&1 && [ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
  VERDICT_LINE="$(grep -m1 -E '^(GREEN|YELLOW|RED) ' "$REPORT" 2>/dev/null || echo "report ready")"
  notify-send -u normal "Validator 24h check-in" "$VERDICT_LINE — see $REPORT"
fi
