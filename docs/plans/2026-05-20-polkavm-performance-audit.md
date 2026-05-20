# PolkaVM Performance Audit and Optimization

Date: 2026-05-20

## Scope

Systematically profile the Neo RISC-V host/guest execution path, identify fixed overhead and execution hotspots, add repeatable benchmarks, apply production-safe optimizations, and verify behavior with the existing compatibility suite.

## Method

- Added fixed-overhead Criterion benchmarks for empty script, RET-only script, PUSH/DROP/RET, and 100 NOP guest execution.
- Corrected collection propagation benchmarks to use the current NeoVM opcode map for `NEWARRAY`, `NEWARRAY0`, `DUP`, `APPEND`, and `SETITEM`.
- Added a `profile_hotspot` host example that repeatedly executes focused scripts (`empty`, `ret`, `nop100`, `arithmetic`, `setitem`, `append`) for sampling with macOS `sample`.
- Sampled empty and append-heavy execution before and after memory-layout changes.
- Rebuilt the embedded guest blob and ran ABI, guest, guest-module, and host test suites.

## Primary Hotspot

The dominant fixed overhead was not NeoVM opcode dispatch. It was PolkaVM memory growth and zeroing triggered by guest writes to mutable statics placed after the 128 MiB allocation arena.

The clearest evidence came from empty-script sampling: an empty execution spent almost all samples under:

```text
polkavm::interpreter::InterpretedInstance::run_impl
  polkavm::zygote::StandardMemory::store_impl_slow
    polkavm::zygote::StandardMemory::rw_data_resize
      __bzero
```

This made even an empty script pay millisecond-scale memory initialization cost. Append-heavy scripts additionally showed normal interpreter handler cost (`load_indirect`, `store_indirect`, arithmetic handlers), but the fixed memory-growth cost was the largest project-wide optimization target because every execution paid it.

## Optimization Applied

- Consolidated guest mutable runtime state into a small `RuntimeState` placed in low `.data` with `#[cfg_attr(target_arch = "riscv32", link_section = ".data.neo_riscv_state")]`.
- Moved syscall request/response scratch buffers into the large `ARENA`, above the allocation region.
- Removed heap-owning static result storage and replaced it with a result pointer/length into arena-allocated bytes that remain valid until the host reads them.
- Reset tracing, panic length, result metadata, and allocator state through the low runtime-state object instead of through high-address statics.
- Added a 1 MiB allocation-arena pre-touch at execution start. This trades one bounded memory growth for many incremental `store_impl_slow` growth events during allocation-heavy execution.
- Kept the 128 MiB allocation headroom required by known mainnet blocks while reducing repeated per-call memory zeroing.

The final RISC-V symbol layout confirms the intended placement:

```text
00037b6c d RUNTIME_STATE
01037e80 b ARENA
```

## Benchmark Results

Representative before/after medians from local Criterion runs:

| Benchmark | Before | After | Change |
| --- | ---: | ---: | ---: |
| `host_overhead_empty_script` | 1.756 ms | 0.184 ms | about 9.5x faster |
| `setitem_50_ops_on_100_elem_array` | 4.845 ms | 3.205 ms | about 34% faster |
| `append_100_ops` | 6.616 ms | 5.263 ms | about 20% faster |
| `arithmetic_1000_ops` | 3.077 ms | 1.100 ms | about 2.8x faster |

The machine was under active background load during some full-suite benchmark runs, so very small codec-only timings and some long control-flow timings should be treated as noisy. The fixed-overhead and collection benchmarks were rerun directly and consistently show the optimization.

## Verification

Passed:

- `RUSTUP_TOOLCHAIN=stable cargo test -p neo-riscv-abi`
- `RUSTUP_TOOLCHAIN=stable cargo test -p neo-riscv-guest`
- `RUSTUP_TOOLCHAIN=stable cargo test -p neo-riscv-guest-module`
- `RUSTUP_TOOLCHAIN=stable cargo test -p neo-riscv-host`
- `RUSTUP_TOOLCHAIN=stable cargo bench -p neo-riscv-host --bench benchmark_harness host_overhead_empty_script -- --warm-up-time 1 --measurement-time 3`
- `RUSTUP_TOOLCHAIN=stable cargo bench -p neo-riscv-host --bench propagate_update_bench -- --warm-up-time 1 --measurement-time 3`
- `RUSTUP_TOOLCHAIN=stable cargo bench -p neo-riscv-host --bench benchmark_harness -- --warm-up-time 1 --measurement-time 3`

## Follow-Up Opportunities

- Run a clean low-load benchmark pass on an otherwise idle machine and store the Criterion report artifacts.
- Add a CI or nightly performance smoke benchmark for `host_overhead_empty_script`, `append_100_ops`, and a mainnet-derived syscall-heavy script.
- Profile interpreter handler mix after fixed overhead removal; the next likely work is reducing guest-side allocation churn in array/map mutation and callback decode paths.
- Evaluate whether result serialization can write directly into the response arena for small halt stacks to avoid one Vec allocation.
