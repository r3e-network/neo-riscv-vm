# neo-riscv-contract-harness Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Test harness for contract-level RISC-V execution and syscall simulation. |
| Inputs | contract module, mock context, expected stack |
| Responsibilities | Initialize test context, Run contract export, Assert state/stack |
| Outputs | test result, trace, failure diagnostics |
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
| `src/lib.rs` | crate root, public exports, and top-level documentation | 8 | 1 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `struct EntryResult` | `src/lib.rs` |
| `fn decode_entry` | `src/lib.rs` |
| `fn decode_context` | `src/lib.rs` |
| `fn encode_result` | `src/lib.rs` |
| `fn get_debug_ptr` | `src/lib.rs` |
| `fn get_debug_len` | `src/lib.rs` |
| `fn reset_debug` | `src/lib.rs` |
| `fn bridge_syscall` | `src/lib.rs` |

## Module and Re-Export Signals

No `mod` or `pub use` declarations were scanned.

## Test Evidence

| Test | File |
| --- | --- |
| `small_boolean_callback_result_survives_assert_top` | `src/lib.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-riscv-abi` | runtime |
| `neo-riscv-guest` | runtime |
| `polkavm-derive` | runtime |
| `postcard` | runtime |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Initialize test context, Run contract export, Assert state/stack.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
