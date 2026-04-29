#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FUZZ_DIR="$PROJECT_DIR/fuzz"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
OUT_ROOT="${OUT_ROOT:-$(mktemp -d /tmp/neo-riscv-cmin-XXXXXX)}"
APPLY_CHANGES=0
PRESERVE_NAMED_SEEDS=1

DEFAULT_TARGETS=(
  opcode_seq
  type_convert
  stack_ops
  exception_handling
  syscall_fuzz
  whole_system_parity
  mem_op
)

TARGETS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply)
      APPLY_CHANGES=1
      shift
      ;;
    --no-preserve-named-seeds)
      PRESERVE_NAMED_SEEDS=0
      shift
      ;;
    --)
      shift
      while [[ $# -gt 0 ]]; do
        TARGETS+=("$1")
        shift
      done
      ;;
    -*)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
    *)
      TARGETS+=("$1")
      shift
      ;;
  esac
done

if [[ ${#TARGETS[@]} -eq 0 ]]; then
  TARGETS=("${DEFAULT_TARGETS[@]}")
fi

mkdir -p "$OUT_ROOT"

is_hash_seed_name() {
  local name="$1"
  [[ "$name" =~ ^[0-9a-f]{40}$ ]]
}

for target in "${TARGETS[@]}"; do
  case "$target" in
    opcode_seq)
      INPUT_DIRS=("$FUZZ_DIR/corpus/opcode_seq")
      PRIMARY_CORPUS_DIR="$FUZZ_DIR/corpus/opcode_seq"
      if [[ -d "$FUZZ_DIR/corpus/opcodes" ]]; then
        INPUT_DIRS+=("$FUZZ_DIR/corpus/opcodes")
      fi
      ;;
    *)
      INPUT_DIRS=("$FUZZ_DIR/corpus/$target")
      PRIMARY_CORPUS_DIR="$FUZZ_DIR/corpus/$target"
      ;;
  esac

  VALID_INPUTS=()
  for dir in "${INPUT_DIRS[@]}"; do
    if [[ -d "$dir" ]]; then
      VALID_INPUTS+=("$dir")
    fi
  done

  if [[ ${#VALID_INPUTS[@]} -eq 0 ]]; then
    echo "Skipping $target: no corpus directories found" >&2
    continue
  fi

  echo "=== Minimizing $target ==="
  cargo +nightly fuzz build "$target" >/dev/null

  OUTPUT_DIR="$OUT_ROOT/$target"
  rm -rf "$OUTPUT_DIR"
  mkdir -p "$OUTPUT_DIR"

  "$FUZZ_DIR/target/$TARGET_TRIPLE/release/$target" -merge=1 "$OUTPUT_DIR" "${VALID_INPUTS[@]}"

  input_count=0
  for dir in "${VALID_INPUTS[@]}"; do
    count=$(find "$dir" -maxdepth 1 -type f | wc -l)
    input_count=$((input_count + count))
  done
  output_count=$(find "$OUTPUT_DIR" -maxdepth 1 -type f | wc -l)

  echo "Input files: $input_count"
  echo "Minimized files: $output_count"
  echo "Output dir: $OUTPUT_DIR"

  if [[ "$APPLY_CHANGES" -eq 1 ]]; then
    mkdir -p "$PRIMARY_CORPUS_DIR"

    while IFS= read -r existing; do
      name="$(basename "$existing")"
      if [[ "$PRESERVE_NAMED_SEEDS" -eq 1 ]] && ! is_hash_seed_name "$name"; then
        continue
      fi

      if [[ ! -f "$OUTPUT_DIR/$name" ]]; then
        rm -f "$existing"
      fi
    done < <(find "$PRIMARY_CORPUS_DIR" -maxdepth 1 -type f | sort)

    while IFS= read -r minimized; do
      name="$(basename "$minimized")"
      destination="$PRIMARY_CORPUS_DIR/$name"
      if [[ ! -f "$destination" ]] || ! cmp -s "$minimized" "$destination"; then
        cp "$minimized" "$destination"
      fi
    done < <(find "$OUTPUT_DIR" -maxdepth 1 -type f | sort)

    applied_count=$(find "$PRIMARY_CORPUS_DIR" -maxdepth 1 -type f | wc -l)
    echo "Applied corpus files: $applied_count"
    echo "Applied to: $PRIMARY_CORPUS_DIR"
  fi
done

echo "Minimized corpora written under: $OUT_ROOT"
