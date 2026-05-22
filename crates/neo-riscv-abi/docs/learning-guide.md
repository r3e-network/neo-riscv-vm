# neo-riscv-abi Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Shared ABI, stack values, codec tags, and opcode metadata re-exports for RISC-V execution. |
| Inputs | shared VM types, host/guest boundary, serialized stack values |
| Responsibilities | Define stable ABI, Re-export shared metadata, Encode/decode stack values |
| Outputs | ABI types, codec helpers, runtime constants |
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

## Source File Map

| File | Role | Public symbols | Tests |
| --- | --- | ---: | ---: |
| `src/lib.rs` | crate root, public exports, and top-level documentation | 0 | 2 |
| `tests/codec_tests.rs` | external behavior or integration test | 0 | 41 |
| `tests/shared_vm_codecs.rs` | external behavior or integration test | 0 | 2 |
| `src/callback_codec.rs` | implementation detail or helper module | 0 | 0 |
| `src/fast_codec.rs` | implementation detail or helper module | 0 | 0 |
| `src/result_codec.rs` | implementation detail or helper module | 0 | 0 |

## Public API Surface

No public Rust symbols were scanned.

## Module and Re-Export Signals

| Signal |
| --- |
| `src/callback_codec.rs: pub use neo_vm_rs::callback_codec::*` |
| `src/fast_codec.rs: pub use neo_vm_rs::fast_codec::*` |
| `src/lib.rs: mod callback_codec` |
| `src/lib.rs: mod fast_codec` |
| `src/lib.rs: mod result_codec` |
| `src/lib.rs: pub use neo_vm_rs::semantics` |
| `src/lib.rs: pub use neo_vm_rs::{     byte_sequence_bytes, byte_sequence_len, concat_byte_sequences, default_value_for_type_tag,     encode_integer, interop_hash, new_array_default_value_for_type_tag,     normalize_stack_item_type_tag, slice_byte_sequence, stack_value_as_bool, stack_value_as_bytes,     stack_value_as_fixed_bytes, stack_value_as_i64, stack_value_as_string, stack_value_as_u32,     stack_value_as_u8, stack_value_into_items, syscall_arg_count, BackendKind, ExecutionResult,     OpCode, StackValue, VmContext, VmState, COMPACT_TAG_ARRAY as TAG_ARRAY,     COMPACT_TAG_BIG_INTEGER as TAG_BIG_INTEGER, COMPACT_TAG_BOOLEAN as TAG_BOOLEAN,     COMPACT_TAG_BUFFER as TAG_BUFFER, COMPACT_TAG_BYTESTRING as TAG_BYTESTRING,     COMPACT_TAG_INTEGER as TAG_INTEGER, COMPACT_TAG_INTEROP as TAG_INTEROP,     COMPACT_TAG_ITERATOR as TAG_ITERATOR, COMPACT_TAG_MAP as TAG_MAP, COMPACT_TAG_NULL as TAG_NULL,     COMPACT_TAG_POINTER as TAG_POINTER, COMPACT_TAG_STRUCT as TAG_STRUCT,     STACK_VALUE_CODEC_TAG_ARRAY, STACK_VALUE_CODEC_TAG_BIG_INTEGER, STACK_VALUE_CODEC_TAG_BOOLEAN,     STACK_VALUE_CODEC_TAG_BUFFER, STACK_VALUE_CODEC_TAG_BYTESTRING, STACK_VALUE_CODEC_TAG_INTEGER,     STACK_VALUE_CODEC_TAG_INTEROP, STACK_VALUE_CODEC_TAG_ITERATOR, STACK_VALUE_CODEC_TAG_MAP,     STACK_VALUE_CODEC_TAG_NULL, STACK_VALUE_CODEC_TAG_POINTER, STACK_VALUE_CODEC_TAG_STRUCT, }` |
| `src/result_codec.rs: pub use neo_vm_rs::result_codec::*` |

## Test Evidence

| Test | File |
| --- | --- |
| `variable_stack_syscalls_keep_the_full_stack` | `src/lib.rs` |
| `create_multisig_account_uses_neovm_descriptor_arguments` | `src/lib.rs` |
| `integer_zero` | `tests/codec_tests.rs` |
| `integer_positive` | `tests/codec_tests.rs` |
| `integer_negative` | `tests/codec_tests.rs` |
| `integer_max` | `tests/codec_tests.rs` |
| `integer_min` | `tests/codec_tests.rs` |
| `biginteger_empty` | `tests/codec_tests.rs` |
| `biginteger_small` | `tests/codec_tests.rs` |
| `biginteger_large` | `tests/codec_tests.rs` |
| `bytestring_empty` | `tests/codec_tests.rs` |
| `bytestring_hello` | `tests/codec_tests.rs` |
| `bytestring_binary` | `tests/codec_tests.rs` |
| `boolean_true` | `tests/codec_tests.rs` |
| `boolean_false` | `tests/codec_tests.rs` |
| `array_empty` | `tests/codec_tests.rs` |
| `array_nested` | `tests/codec_tests.rs` |
| `struct_empty` | `tests/codec_tests.rs` |
| `struct_nested` | `tests/codec_tests.rs` |
| `map_empty` | `tests/codec_tests.rs` |
| `map_with_entries` | `tests/codec_tests.rs` |
| `interop_zero` | `tests/codec_tests.rs` |
| `interop_max` | `tests/codec_tests.rs` |
| `iterator_zero` | `tests/codec_tests.rs` |
| `iterator_42` | `tests/codec_tests.rs` |
| `null_value` | `tests/codec_tests.rs` |
| `pointer_zero` | `tests/codec_tests.rs` |
| `pointer_negative` | `tests/codec_tests.rs` |
| `pointer_max` | `tests/codec_tests.rs` |
| `error_result_round_trip` | `tests/codec_tests.rs` |
| `error_result_empty_message` | `tests/codec_tests.rs` |
| `interop_hash_platform` | `tests/codec_tests.rs` |
| `interop_hash_contract_call` | `tests/codec_tests.rs` |
| `interop_hash_is_sha256_first_4_bytes_le` | `tests/codec_tests.rs` |
| `empty_stack_round_trip` | `tests/codec_tests.rs` |
| `multi_item_stack_round_trip` | `tests/codec_tests.rs` |
| `truncated_bytes_returns_error` | `tests/codec_tests.rs` |
| `invalid_tag_byte_returns_error` | `tests/codec_tests.rs` |
| `invalid_stack_value_tag_returns_error` | `tests/codec_tests.rs` |
| `trailing_bytes_returns_error` | `tests/codec_tests.rs` |
| `completely_empty_input_returns_error` | `tests/codec_tests.rs` |
| `decode_rejects_excessive_nesting` | `tests/codec_tests.rs` |
| `decode_rejects_excessive_collection_length` | `tests/codec_tests.rs` |
| `callback_codec_is_reexported_from_shared_vm_crate` | `tests/shared_vm_codecs.rs` |
| `result_codec_is_reexported_from_shared_vm_crate` | `tests/shared_vm_codecs.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-vm-rs` | runtime |
| `sha2` | test |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.
2. Read `tests/codec_tests.rs`: external behavior or integration test.
3. Read `tests/shared_vm_codecs.rs`: external behavior or integration test.
4. Read `src/callback_codec.rs`: implementation detail or helper module.
5. Read `src/fast_codec.rs`: implementation detail or helper module.
6. Read `src/result_codec.rs`: implementation detail or helper module.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Define stable ABI, Re-export shared metadata, Encode/decode stack values.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
