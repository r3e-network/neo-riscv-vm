#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FUZZ_DIR="$PROJECT_DIR/fuzz"

FUZZ_TARGETS=(
  opcode_seq
  type_convert
  stack_ops
  exception_handling
  syscall_fuzz
  whole_system_parity
  mem_op
)

DEFAULT_TIME_PER_TARGET=120
DEFAULT_RUNS_PER_TARGET=100
DEFAULT_FUZZ_SEED=42

TIME_PER_TARGET="${TIME_PER_TARGET:-$DEFAULT_TIME_PER_TARGET}"
RUNS_PER_TARGET="${RUNS_PER_TARGET:-$DEFAULT_RUNS_PER_TARGET}"
FUZZ_SEED="${FUZZ_SEED:-$DEFAULT_FUZZ_SEED}"

cd "$FUZZ_DIR"

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install it with: cargo install cargo-fuzz" >&2
  exit 1
fi

if ! cargo +nightly --version >/dev/null 2>&1; then
  echo "cargo +nightly is required for instrumented fuzzing. Install it with: rustup toolchain install nightly" >&2
  exit 1
fi

export RUST_BACKTRACE=1

for target in "${FUZZ_TARGETS[@]}"; do
  ARGS=(
    "-max_total_time=$TIME_PER_TARGET"
    "-runs=$RUNS_PER_TARGET"
    "-seed=$FUZZ_SEED"
  )

  CORPUS_ARGS=()
  if [[ "$target" == "opcode_seq" ]]; then
    CORPUS_DIR="$FUZZ_DIR/corpus/opcodes"
    if [[ -d "$CORPUS_DIR" ]]; then
      CORPUS_ARGS+=("$CORPUS_DIR")
    else
      echo "Opcode corpus missing at $CORPUS_DIR, running opcode_seq without a seed corpus" >&2
    fi
  fi

  echo "=== Running $target (time=${TIME_PER_TARGET}s runs=${RUNS_PER_TARGET} seed=$FUZZ_SEED) ==="
  cargo +nightly fuzz run "$target" -- "${ARGS[@]}" "${CORPUS_ARGS[@]}"
done
