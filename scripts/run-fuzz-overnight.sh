#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
MATRIX_SCRIPT="$SCRIPT_DIR/run-fuzz-seed-matrix.sh"

OUT_DIR="${OUT_DIR:-$(mktemp -d /tmp/neo-riscv-fuzz-overnight-XXXXXX)}"
TIME_PER_TARGET="${TIME_PER_TARGET:-1800}"
RUNS_PER_TARGET="${RUNS_PER_TARGET:-0}"
FUZZ_SEEDS="${FUZZ_SEEDS:-123 321 987 2027}"

DEFAULT_TARGETS=(
  stack_ops
  whole_system_parity
)

if [[ $# -eq 0 ]]; then
  TARGETS=("${DEFAULT_TARGETS[@]}")
else
  TARGETS=("$@")
fi

mkdir -p "$OUT_DIR"

OUT_DIR="$OUT_DIR" \
TIME_PER_TARGET="$TIME_PER_TARGET" \
RUNS_PER_TARGET="$RUNS_PER_TARGET" \
FUZZ_SEEDS="$FUZZ_SEEDS" \
"$MATRIX_SCRIPT" "${TARGETS[@]}" >/dev/null

summary_tsv="$OUT_DIR/summary.tsv"
summary_md="$OUT_DIR/summary.md"

{
  echo "# Fuzz Overnight Summary"
  echo
  echo "- Output dir: \`$OUT_DIR\`"
  echo "- Targets: \`${TARGETS[*]}\`"
  echo "- Seeds: \`$FUZZ_SEEDS\`"
  echo "- Time per target: \`${TIME_PER_TARGET}s\`"
  echo "- Runs per target: \`${RUNS_PER_TARGET}\`"
  echo
  echo "| Target | Seed | Status | Runs | Seconds | Cov | FT | Corpus | RSS | Log |"
  echo "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
  tail -n +2 "$summary_tsv" | while IFS=$'\t' read -r target seed time runs status done_runs done_seconds cov ft corp rss log; do
    echo "| \`$target\` | \`$seed\` | \`$status\` | \`${done_runs:-}\` | \`${done_seconds:-}\` | \`${cov:-}\` | \`${ft:-}\` | \`${corp:-}\` | \`${rss:-}\` | \`$log\` |"
  done
} > "$summary_md"

echo "Summary TSV: $summary_tsv"
echo "Summary MD: $summary_md"
echo "Output dir: $OUT_DIR"
