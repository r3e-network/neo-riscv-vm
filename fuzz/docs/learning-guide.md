# neo-riscv-fuzz Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Fuzzing support for RISC-V VM execution, ABI codecs, and host/guest boundaries. |
| Inputs | seed corpus, generated opcodes, mutated stack values |
| Responsibilities | Generate valid scripts, Exercise codecs, Find host/guest mismatches |
| Outputs | regression seed, crash case, coverage signal |
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
| `src/generators/value_gen.rs` | implementation detail or helper module | 4 | 4 |
| `src/generators/opcode_gen.rs` | opcode metadata, pricing, or canonical decode rules | 3 | 4 |
| `src/stack_ops_builder.rs` | implementation detail or helper module | 1 | 2 |
| `src/whole_system_parity.rs` | implementation detail or helper module | 0 | 4 |
| `src/syscall_fuzz.rs` | fuzzing harness and adversarial input exploration | 0 | 1 |
| `src/exception_handling.rs` | implementation detail or helper module | 0 | 0 |
| `src/generators.rs` | implementation detail or helper module | 0 | 0 |
| `src/mem_op.rs` | implementation detail or helper module | 0 | 0 |
| `src/opcode_seq.rs` | opcode metadata, pricing, or canonical decode rules | 0 | 0 |
| `src/stack_ops.rs` | implementation detail or helper module | 0 | 0 |
| `src/type_convert.rs` | implementation detail or helper module | 0 | 0 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `fn is_valid_opcode` | `src/generators/opcode_gen.rs` |
| `fn requires_immediate` | `src/generators/opcode_gen.rs` |
| `fn generate_valid_script` | `src/generators/opcode_gen.rs` |
| `fn generate_integer` | `src/generators/value_gen.rs` |
| `fn generate_big_integer` | `src/generators/value_gen.rs` |
| `fn generate_bytestring` | `src/generators/value_gen.rs` |
| `fn generate_stack_value` | `src/generators/value_gen.rs` |
| `fn run_script` | `src/lib.rs` |
| `fn run_with_stack` | `src/lib.rs` |
| `fn assert_invariants` | `src/lib.rs` |
| `struct NoOpSyscall` | `src/lib.rs` |
| `fn check_stack_values` | `src/lib.rs` |
| `struct SimpleRng` | `src/lib.rs` |
| `fn new` | `src/lib.rs` |
| `fn next` | `src/lib.rs` |
| `fn build_stack_ops_script` | `src/stack_ops_builder.rs` |

## Module and Re-Export Signals

| Signal |
| --- |
| `src/generators.rs: mod opcode_gen` |
| `src/generators.rs: mod value_gen` |
| `src/generators.rs: pub use opcode_gen::*` |
| `src/generators.rs: pub use value_gen::*` |
| `src/lib.rs: mod generators` |
| `src/lib.rs: mod stack_ops_builder` |
| `src/stack_ops.rs: mod stack_ops_builder` |

## Test Evidence

| Test | File |
| --- | --- |
| `test_opcode_ranges` | `src/generators/opcode_gen.rs` |
| `test_is_valid_opcode` | `src/generators/opcode_gen.rs` |
| `test_requires_immediate` | `src/generators/opcode_gen.rs` |
| `test_generate_valid_script` | `src/generators/opcode_gen.rs` |
| `test_generate_integer` | `src/generators/value_gen.rs` |
| `test_generate_big_integer` | `src/generators/value_gen.rs` |
| `test_generate_bytestring` | `src/generators/value_gen.rs` |
| `test_generate_stack_value` | `src/generators/value_gen.rs` |
| `test_no_op_syscall` | `src/lib.rs` |
| `default_stack_ops_script_halts` | `src/stack_ops_builder.rs` |
| `stack_ops_builder_does_not_inject_non_stack_opcodes` | `src/stack_ops_builder.rs` |
| `test_syscall_fuzzing` | `src/syscall_fuzz.rs` |
| `expected_trace` | `src/whole_system_parity.rs` |
| `expected_result` | `src/whole_system_parity.rs` |
| `platform_seed_matches_direct_guest_and_host_path` | `src/whole_system_parity.rs` |
| `storage_seed_matches_direct_guest_and_host_path` | `src/whole_system_parity.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `libfuzzer-sys` | runtime |
| `neo-riscv-abi` | runtime |
| `neo-riscv-guest` | runtime |
| `neo-riscv-host` | runtime |
| `serde` | runtime |
| `serde_json` | runtime |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.
2. Read `src/generators/value_gen.rs`: implementation detail or helper module.
3. Read `src/generators/opcode_gen.rs`: opcode metadata, pricing, or canonical decode rules.
4. Read `src/stack_ops_builder.rs`: implementation detail or helper module.
5. Read `src/whole_system_parity.rs`: implementation detail or helper module.
6. Read `src/syscall_fuzz.rs`: fuzzing harness and adversarial input exploration.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Generate valid scripts, Exercise codecs, Find host/guest mismatches.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
