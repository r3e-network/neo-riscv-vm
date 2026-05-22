# neo-riscv-guest-module Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | PolkaVM guest module entrypoint that packages the guest runtime into executable RISC-V code. |
| Inputs | PolkaVM imports, guest runtime, encoded execution input |
| Responsibilities | Expose exports, Call guest runtime, Return encoded output |
| Outputs | guest.polkavm, encoded output, host callback calls |
| Consumers | RISC-V host, Neo N4 L2 node, developer tooling |

## Visual Reading Order

| Step | Diagram | Use it to learn |
| ---: | --- | --- |
| 1 | [Position](figures/position.svg) | Why this crate exists and where it sits in Neo N4. |
| 2 | [Principles](figures/principles.svg) | The invariants and boundaries this crate must protect. |
| 3 | [Module map](figures/module-map.svg) | Which files are the best entry points. |
| 4 | [Public API surface](figures/api-surface.svg) | Which exported symbols form the crate contract. |
| 5 | [Architecture](figures/architecture.svg) | How inputs, internal components, dependencies, and outputs connect. |
| 6 | [Workflow](figures/workflow.svg) | The normal execution path. |
| 7 | [Dataflow](figures/dataflow.svg) | How data is transformed across the crate boundary. |
| 8 | [Test evidence](figures/test-map.svg) | Which tests protect the behavior. |
| 9 | [Dependency map](figures/dependency-map.svg) | Which dependencies are runtime, test, or build-only. |
| 10 | [Implementation atlas](figures/implementation-atlas.svg) | A dense one-page map of purpose, source entrypoints, API, workflow, dataflow, dependencies, tests, and change checks. |

## Source File Map

| File | Role | Public symbols | Tests |
| --- | --- | ---: | ---: |
| `src/main.rs` | binary or CLI entrypoint | 26 | 3 |
| `src/mem_intrinsics.rs` | implementation detail or helper module | 0 | 0 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `fn _start` | `src/main.rs` |
| `fn main` | `src/main.rs` |
| `fn alloc` | `src/main.rs` |
| `fn get_result_ptr` | `src/main.rs` |
| `fn get_result_len` | `src/main.rs` |
| `fn get_panic_ptr` | `src/main.rs` |
| `fn get_panic_len` | `src/main.rs` |
| `fn get_trace_res_len` | `src/main.rs` |
| `fn get_trace_res_head_ptr` | `src/main.rs` |
| `fn get_trace_syscall_stage` | `src/main.rs` |
| `fn get_trace_syscall_api` | `src/main.rs` |
| `fn get_trace_syscall_ip` | `src/main.rs` |
| `fn get_last_interpreter_ip` | `src/main.rs` |
| `fn get_last_result_stage` | `src/main.rs` |
| `fn get_last_result_stack_len` | `src/main.rs` |
| `fn get_last_result_limit` | `src/main.rs` |
| `fn get_trace_req_len` | `src/main.rs` |
| `fn get_trace_stack_items` | `src/main.rs` |
| `fn get_allocator_peak` | `src/main.rs` |
| `fn get_allocator_fail_count` | `src/main.rs` |
| `fn get_allocator_fail_size` | `src/main.rs` |
| `fn get_allocator_fail_align` | `src/main.rs` |
| `fn execute` | `src/main.rs` |
| `fn execute_with_result_limit` | `src/main.rs` |
| `fn set_result_limit` | `src/main.rs` |
| `fn execute_with_initializer` | `src/main.rs` |

## Module and Re-Export Signals

| Signal |
| --- |
| `src/main.rs: mod mem_intrinsics` |

## Test Evidence

| Test | File |
| --- | --- |
| `mainnet_470449_requires_heap_headroom_above_16_mib` | `src/main.rs` |
| `mainnet_2368696_requires_heap_headroom_above_32_mib` | `src/main.rs` |
| `mainnet_2655903_requires_heap_headroom_above_64_mib` | `src/main.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-riscv-abi` | runtime |
| `neo-riscv-guest` | runtime |
| `polkavm-derive` | runtime |
| `postcard` | runtime |

## Suggested Reading Path

1. Read `src/main.rs`: binary or CLI entrypoint.
2. Read `src/mem_intrinsics.rs`: implementation detail or helper module.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Expose exports, Call guest runtime, Return encoded output.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
