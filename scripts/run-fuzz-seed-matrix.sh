#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
FUZZ_DIR="$PROJECT_DIR/fuzz"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
OUT_DIR="${OUT_DIR:-$(mktemp -d /tmp/neo-riscv-fuzz-matrix-XXXXXX)}"
TIME_PER_TARGET="${TIME_PER_TARGET:-60}"
RUNS_PER_TARGET="${RUNS_PER_TARGET:-0}"
FUZZ_SEEDS="${FUZZ_SEEDS:-123 321 987}"

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

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is required. Install it with: cargo install cargo-fuzz" >&2
  exit 1
fi

if ! cargo +nightly --version >/dev/null 2>&1; then
  echo "cargo +nightly is required. Install it with: rustup toolchain install nightly" >&2
  exit 1
fi

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

materialize_seed_corpus() {
  local target="$1"
  local seed="$2"
  local seed_root="$OUT_DIR/work_corpus/$target/seed${seed}"
  mkdir -p "$seed_root"

  mapfile -t source_dirs < <(target_corpus_sources "$target")
  local source_dir
  for source_dir in "${source_dirs[@]}"; do
    if [[ ! -d "$source_dir" ]]; then
      continue
    fi

    local leaf
    leaf="$(basename "$source_dir")"
    local dest_dir="$seed_root/$leaf"
    mkdir -p "$dest_dir"
    cp "$source_dir"/* "$dest_dir"/ 2>/dev/null || true
  done

  for source_dir in "${source_dirs[@]}"; do
    if [[ ! -d "$source_dir" ]]; then
      continue
    fi

    local leaf
    leaf="$(basename "$source_dir")"
    printf '%s\n' "$seed_root/$leaf"
  done
}

summary_file="$OUT_DIR/summary.tsv"
printf 'target\tseed\ttime_per_target\truns_per_target\tstatus\tdone_runs\tdone_seconds\tcov\tft\tcorp\trss\tlog\n' > "$summary_file"

for target in "${TARGETS[@]}"; do
  echo "=== Building $target ==="
  build_target "$target"

  artifact_dir="$OUT_DIR/artifacts/$target"
  mkdir -p "$artifact_dir"

  for seed in $FUZZ_SEEDS; do
    mapfile -t corpus_args < <(materialize_seed_corpus "$target" "$seed")
    log_file="$OUT_DIR/${target}-seed${seed}.log"
    args=(
      "-artifact_prefix=${artifact_dir}/"
      "-max_total_time=${TIME_PER_TARGET}"
      "-seed=${seed}"
    )

    if [[ "$RUNS_PER_TARGET" != "0" ]]; then
      args+=("-runs=${RUNS_PER_TARGET}")
    fi

    echo "=== Running $target seed=${seed} time=${TIME_PER_TARGET}s runs=${RUNS_PER_TARGET} ==="
    set +e
    "$FUZZ_DIR/target/$TARGET_TRIPLE/release/$target" "${args[@]}" "${corpus_args[@]}" 2>&1 | tee "$log_file"
    run_exit=${PIPESTATUS[0]}
    set -e

    status="ok"
    if [[ "$run_exit" -ne 0 ]]; then
      status="failed"
    fi

    done_line="$(grep -E '^Done [0-9]+ runs in [0-9]+ second\(s\)$' "$log_file" | tail -n1 || true)"
    done_runs=""
    done_seconds=""
    if [[ -n "$done_line" ]]; then
      done_runs="$(sed -E 's/^Done ([0-9]+) runs in ([0-9]+) second\(s\)$/\1/' <<<"$done_line")"
      done_seconds="$(sed -E 's/^Done ([0-9]+) runs in ([0-9]+) second\(s\)$/\2/' <<<"$done_line")"
    fi

    final_line="$(grep -E '#[0-9]+[[:space:]]+DONE[[:space:]]+cov:' "$log_file" | tail -n1 || true)"
    cov=""
    ft=""
    corp=""
    rss=""
    if [[ -n "$final_line" ]]; then
      cov="$(sed -nE 's/.*cov: ([0-9]+).*/\1/p' <<<"$final_line")"
      ft="$(sed -nE 's/.*ft: ([0-9]+).*/\1/p' <<<"$final_line")"
      corp="$(sed -nE 's/.*corp: ([0-9]+\/[0-9]+b).*/\1/p' <<<"$final_line")"
      rss="$(sed -nE 's/.*rss: ([0-9]+Mb).*/\1/p' <<<"$final_line")"
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$target" "$seed" "$TIME_PER_TARGET" "$RUNS_PER_TARGET" "$status" \
      "$done_runs" "$done_seconds" "$cov" "$ft" "$corp" "$rss" "$log_file" \
      >> "$summary_file"

    if [[ "$run_exit" -ne 0 ]]; then
      echo "Run failed for $target seed=$seed; see $log_file" >&2
      exit "$run_exit"
    fi
  done
done

echo "Logs written under: $OUT_DIR"
echo "Summary: $summary_file"
