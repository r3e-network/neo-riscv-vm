#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FUZZ_DIR="$PROJECT_DIR/fuzz"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
WORK_ROOT="$(mktemp -d /tmp/neo-riscv-bounded-fuzz-XXXXXX)"
WORK_CORPUS_ROOT="$WORK_ROOT/corpus"
WORK_ARTIFACT_ROOT="$WORK_ROOT/artifacts"
PRESERVE_WORK_ROOT_ON_EXIT=0

cleanup() {
  if [[ "$PRESERVE_WORK_ROOT_ON_EXIT" -eq 0 ]]; then
    rm -rf "$WORK_ROOT"
  fi
}
trap cleanup EXIT

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

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install it with: cargo install cargo-fuzz" >&2
  exit 1
fi

if ! cargo +nightly --version >/dev/null 2>&1; then
  echo "cargo +nightly is required for instrumented fuzzing. Install it with: rustup toolchain install nightly" >&2
  exit 1
fi

export RUST_BACKTRACE=1

build_target() {
  local target="$1"
  cargo +nightly fuzz build "$target" >/dev/null
}

target_corpus_sources() {
  local target="$1"
  case "$target" in
    opcode_seq)
      if [[ -d "$FUZZ_DIR/corpus/opcodes" ]]; then
        printf '%s\n' "$FUZZ_DIR/corpus/opcodes"
      fi
      printf '%s\n' "$FUZZ_DIR/corpus/opcode_seq"
      ;;
    *)
      printf '%s\n' "$FUZZ_DIR/corpus/$target"
      ;;
  esac
}

materialize_work_corpus() {
  local target="$1"
  local work_root="$WORK_CORPUS_ROOT/$target"
  mkdir -p "$work_root"

  mapfile -t source_dirs < <(target_corpus_sources "$target")
  local source_dir
  for source_dir in "${source_dirs[@]}"; do
    if [[ ! -d "$source_dir" ]]; then
      continue
    fi

    local leaf
    leaf="$(basename "$source_dir")"
    local dest_dir="$work_root/$leaf"
    mkdir -p "$dest_dir"
    cp "$source_dir"/* "$dest_dir"/ 2>/dev/null || true
  done

  for source_dir in "${source_dirs[@]}"; do
    if [[ ! -d "$source_dir" ]]; then
      continue
    fi

    local leaf
    leaf="$(basename "$source_dir")"
    printf '%s\n' "$work_root/$leaf"
  done
}

for target in "${FUZZ_TARGETS[@]}"; do
  build_target "$target"

  ARGS=(
    "-artifact_prefix=$WORK_ARTIFACT_ROOT/$target/"
    "-max_total_time=$TIME_PER_TARGET"
    "-runs=$RUNS_PER_TARGET"
    "-seed=$FUZZ_SEED"
  )

  mapfile -t CORPUS_ARGS < <(materialize_work_corpus "$target")
  mkdir -p "$WORK_ARTIFACT_ROOT/$target"

  echo "=== Running $target (time=${TIME_PER_TARGET}s runs=${RUNS_PER_TARGET} seed=$FUZZ_SEED) ==="
  if ! "$FUZZ_DIR/target/$TARGET_TRIPLE/release/$target" "${ARGS[@]}" "${CORPUS_ARGS[@]}"; then
    PRESERVE_WORK_ROOT_ON_EXIT=1
    echo "Fuzz target $target failed; preserved temporary inputs/artifacts under: $WORK_ROOT" >&2
    exit 1
  fi
done
