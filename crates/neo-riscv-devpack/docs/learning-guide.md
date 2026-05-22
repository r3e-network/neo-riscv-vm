# neo-riscv-devpack Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Developer packaging utilities for compiling and preparing RISC-V Neo contracts. |
| Inputs | contract source, template config, toolchain settings |
| Responsibilities | Build artifacts, Validate metadata, Package deployment files |
| Outputs | contract package, manifest, developer diagnostics |
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
| `src/lib.rs` | crate root, public exports, and top-level documentation | 0 | 0 |
| `src/api_ids.rs` | implementation detail or helper module | 43 | 0 |
| `src/syscalls.rs` | host syscall contract and dispatch boundary | 34 | 0 |
| `src/native/std_lib.rs` | implementation detail or helper module | 13 | 0 |
| `src/storage.rs` | implementation detail or helper module | 11 | 0 |
| `src/native/neo_token.rs` | implementation detail or helper module | 10 | 0 |
| `src/native/policy.rs` | implementation detail or helper module | 9 | 0 |
| `src/native/notary.rs` | implementation detail or helper module | 8 | 0 |
| `src/codec.rs` | implementation detail or helper module | 7 | 0 |
| `src/native/crypto_lib.rs` | implementation detail or helper module | 7 | 0 |
| `src/events.rs` | implementation detail or helper module | 6 | 0 |
| `src/native/gas_token.rs` | implementation detail or helper module | 6 | 0 |
| `src/native/ledger.rs` | implementation detail or helper module | 6 | 0 |
| `src/native/mod.rs` | implementation detail or helper module | 5 | 1 |
| `src/native/contract_management.rs` | implementation detail or helper module | 5 | 0 |
| `src/native/treasury.rs` | implementation detail or helper module | 4 | 0 |
| `src/parser.rs` | implementation detail or helper module | 4 | 0 |
| `tests/native_wrappers_test.rs` | external behavior or integration test | 1 | 9 |
| `tests/syscalls_test.rs` | external behavior or integration test | 1 | 9 |
| `src/signing.rs` | implementation detail or helper module | 3 | 0 |
| `src/types.rs` | implementation detail or helper module | 3 | 0 |
| `src/native/oracle.rs` | implementation detail or helper module | 2 | 0 |
| `src/native/role_management.rs` | implementation detail or helper module | 2 | 0 |
| `tests/syscalls_shape_test.rs` | external behavior or integration test | 1 | 2 |
| `src/ffi.rs` | implementation detail or helper module | 1 | 0 |
| `tests/api_ids_test.rs` | external behavior or integration test | 0 | 1 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `const STORAGE_GET_CONTEXT` | `src/api_ids.rs` |
| `const STORAGE_GET_READONLY_CONTEXT` | `src/api_ids.rs` |
| `const STORAGE_AS_READ_ONLY` | `src/api_ids.rs` |
| `const STORAGE_GET` | `src/api_ids.rs` |
| `const STORAGE_PUT` | `src/api_ids.rs` |
| `const STORAGE_DELETE` | `src/api_ids.rs` |
| `const STORAGE_FIND` | `src/api_ids.rs` |
| `const STORAGE_LOCAL_GET` | `src/api_ids.rs` |
| `const STORAGE_LOCAL_PUT` | `src/api_ids.rs` |
| `const STORAGE_LOCAL_DELETE` | `src/api_ids.rs` |
| `const STORAGE_LOCAL_FIND` | `src/api_ids.rs` |
| `const CONTRACT_CALL` | `src/api_ids.rs` |
| `const CONTRACT_CREATE` | `src/api_ids.rs` |
| `const CONTRACT_UPDATE` | `src/api_ids.rs` |
| `const CONTRACT_GET_CALL_FLAGS` | `src/api_ids.rs` |
| `const CONTRACT_CREATE_STANDARD_ACCOUNT` | `src/api_ids.rs` |
| `const CONTRACT_CREATE_MULTISIG_ACCOUNT` | `src/api_ids.rs` |
| `const CONTRACT_NATIVE_ON_PERSIST` | `src/api_ids.rs` |
| `const CONTRACT_NATIVE_POST_PERSIST` | `src/api_ids.rs` |
| `const RUNTIME_PLATFORM` | `src/api_ids.rs` |
| `const RUNTIME_GET_TRIGGER` | `src/api_ids.rs` |
| `const RUNTIME_GET_NETWORK` | `src/api_ids.rs` |
| `const RUNTIME_GET_ADDRESS_VERSION` | `src/api_ids.rs` |
| `const RUNTIME_GET_SCRIPT_CONTAINER` | `src/api_ids.rs` |
| `const RUNTIME_GET_EXECUTING_SCRIPT_HASH` | `src/api_ids.rs` |
| `const RUNTIME_GET_CALLING_SCRIPT_HASH` | `src/api_ids.rs` |
| `const RUNTIME_GET_ENTRY_SCRIPT_HASH` | `src/api_ids.rs` |
| `const RUNTIME_GET_TIME` | `src/api_ids.rs` |
| `const RUNTIME_GET_INVOCATION_COUNTER` | `src/api_ids.rs` |
| `const RUNTIME_GAS_LEFT` | `src/api_ids.rs` |
| `const RUNTIME_GET_RANDOM` | `src/api_ids.rs` |
| `const RUNTIME_CURRENT_SIGNERS` | `src/api_ids.rs` |
| `const RUNTIME_CHECK_WITNESS` | `src/api_ids.rs` |
| `const RUNTIME_NOTIFY` | `src/api_ids.rs` |
| `const RUNTIME_LOG` | `src/api_ids.rs` |
| `const RUNTIME_GET_NOTIFICATIONS` | `src/api_ids.rs` |
| `const RUNTIME_BURN_GAS` | `src/api_ids.rs` |
| `const RUNTIME_LOAD_SCRIPT` | `src/api_ids.rs` |
| `const CRYPTO_CHECK_SIG` | `src/api_ids.rs` |
| `const CRYPTO_CHECK_MULTISIG` | `src/api_ids.rs` |
| `const ITERATOR_NEXT` | `src/api_ids.rs` |
| `const ITERATOR_VALUE` | `src/api_ids.rs` |
| `const CRYPTO_VERIFY_SIGNATURE` | `src/api_ids.rs` |
| `fn encode_string_params` | `src/codec.rs` |
| `fn encode_int_params` | `src/codec.rs` |
| `fn decode_string_result` | `src/codec.rs` |
| `fn decode_int_result` | `src/codec.rs` |
| `fn decode_bool_result` | `src/codec.rs` |
| `fn encode_bytes` | `src/codec.rs` |
| `fn decode_bytes_result` | `src/codec.rs` |
| `fn notify` | `src/events.rs` |
| `fn notify_string` | `src/events.rs` |
| `fn notify_int` | `src/events.rs` |
| `fn notify_args` | `src/events.rs` |
| `fn notify_key_value` | `src/events.rs` |
| `fn notify_two_strings` | `src/events.rs` |
| `fn invoke_host_call` | `src/ffi.rs` |
| `const CONTRACT_MANAGEMENT_HASH` | `src/native/contract_management.rs` |
| `fn contract_deploy` | `src/native/contract_management.rs` |
| `fn contract_update` | `src/native/contract_management.rs` |
| `fn contract_destroy` | `src/native/contract_management.rs` |
| `fn contract_get_contract` | `src/native/contract_management.rs` |
| `const CRYPTO_LIB_HASH` | `src/native/crypto_lib.rs` |
| `fn crypto_sha256` | `src/native/crypto_lib.rs` |
| `fn crypto_ripemd160` | `src/native/crypto_lib.rs` |
| `fn crypto_verify_with_ecdsa` | `src/native/crypto_lib.rs` |
| `fn crypto_murmur32` | `src/native/crypto_lib.rs` |
| `fn crypto_keccak256` | `src/native/crypto_lib.rs` |
| `fn crypto_verify_with_ed25519` | `src/native/crypto_lib.rs` |
| `const GAS_TOKEN_HASH` | `src/native/gas_token.rs` |
| `fn gas_balance_of` | `src/native/gas_token.rs` |
| `fn gas_transfer` | `src/native/gas_token.rs` |
| `fn gas_symbol` | `src/native/gas_token.rs` |
| `fn gas_decimals` | `src/native/gas_token.rs` |
| `fn gas_total_supply` | `src/native/gas_token.rs` |
| `const LEDGER_CONTRACT_HASH` | `src/native/ledger.rs` |
| `fn ledger_get_block` | `src/native/ledger.rs` |
| `fn ledger_get_transaction` | `src/native/ledger.rs` |
| `fn ledger_current_index` | `src/native/ledger.rs` |
| `fn ledger_get_transaction_height` | `src/native/ledger.rs` |
| `fn ledger_current_hash` | `src/native/ledger.rs` |
| `const CALL_FLAGS_READ_ONLY` | `src/native/mod.rs` |
| `fn build_contract_call_stack` | `src/native/mod.rs` |
| `fn call_native` | `src/native/mod.rs` |
| `fn call_native_read_only` | `src/native/mod.rs` |
| `fn call_native_with_flags` | `src/native/mod.rs` |
| `const NEO_TOKEN_HASH` | `src/native/neo_token.rs` |
| `fn neo_balance_of` | `src/native/neo_token.rs` |
| `fn neo_transfer` | `src/native/neo_token.rs` |
| `fn neo_get_candidates` | `src/native/neo_token.rs` |
| `fn neo_register_candidate` | `src/native/neo_token.rs` |
| `fn neo_vote` | `src/native/neo_token.rs` |
| `fn neo_unclaimed_gas` | `src/native/neo_token.rs` |
| `fn neo_symbol` | `src/native/neo_token.rs` |
| `fn neo_decimals` | `src/native/neo_token.rs` |
| `fn neo_total_supply` | `src/native/neo_token.rs` |
| `const NOTARY_HASH` | `src/native/notary.rs` |
| `fn notary_balance_of` | `src/native/notary.rs` |
| `fn notary_expiration_of` | `src/native/notary.rs` |
| `fn notary_get_max_not_valid_before_delta` | `src/native/notary.rs` |
| `fn notary_lock_deposit_until` | `src/native/notary.rs` |
| `fn notary_withdraw` | `src/native/notary.rs` |
| `fn notary_verify` | `src/native/notary.rs` |
| `fn notary_set_max_not_valid_before_delta` | `src/native/notary.rs` |
| `const ORACLE_CONTRACT_HASH` | `src/native/oracle.rs` |
| `fn oracle_request` | `src/native/oracle.rs` |
| `const POLICY_CONTRACT_HASH` | `src/native/policy.rs` |
| `fn policy_get_fee_per_byte` | `src/native/policy.rs` |
| `fn policy_get_exec_fee_factor` | `src/native/policy.rs` |
| `fn policy_get_storage_price` | `src/native/policy.rs` |
| `fn policy_is_blocked` | `src/native/policy.rs` |
| `fn policy_get_attribute_fee` | `src/native/policy.rs` |
| `fn policy_get_milliseconds_per_block` | `src/native/policy.rs` |
| `fn policy_get_max_valid_until_block_increment` | `src/native/policy.rs` |
| `fn policy_get_max_traceable_blocks` | `src/native/policy.rs` |
| `const ROLE_MANAGEMENT_HASH` | `src/native/role_management.rs` |
| `fn role_get_designated_by_role` | `src/native/role_management.rs` |
| `const STD_LIB_HASH` | `src/native/std_lib.rs` |
| `fn stdlib_serialize_stack_item` | `src/native/std_lib.rs` |
| `fn stdlib_deserialize_stack_item` | `src/native/std_lib.rs` |
| `fn stdlib_serialize` | `src/native/std_lib.rs` |
| `fn stdlib_deserialize` | `src/native/std_lib.rs` |
| `fn stdlib_json_serialize` | `src/native/std_lib.rs` |
| `fn stdlib_json_deserialize` | `src/native/std_lib.rs` |
| `fn stdlib_base64_encode` | `src/native/std_lib.rs` |
| `fn stdlib_base64_decode` | `src/native/std_lib.rs` |
| `fn stdlib_itoa` | `src/native/std_lib.rs` |
| `fn stdlib_atoi` | `src/native/std_lib.rs` |
| `fn stdlib_base58_encode` | `src/native/std_lib.rs` |
| `fn stdlib_base58_decode` | `src/native/std_lib.rs` |
| `const TREASURY_HASH` | `src/native/treasury.rs` |
| `fn treasury_verify` | `src/native/treasury.rs` |
| `fn treasury_on_nep17_payment` | `src/native/treasury.rs` |
| `fn treasury_on_nep11_payment` | `src/native/treasury.rs` |
| `fn parse_stack_value` | `src/parser.rs` |
| `fn parse_string_result` | `src/parser.rs` |
| `fn parse_int_result` | `src/parser.rs` |
| `fn format_stack_value` | `src/parser.rs` |
| `fn verify_signature` | `src/signing.rs` |
| `fn check_witness` | `src/signing.rs` |
| `fn verify_multisig` | `src/signing.rs` |
| `fn get_context` | `src/storage.rs` |
| `fn get_readonly_context` | `src/storage.rs` |
| `fn as_read_only` | `src/storage.rs` |
| `fn get` | `src/storage.rs` |
| `fn put` | `src/storage.rs` |
| `fn delete` | `src/storage.rs` |
| `fn find` | `src/storage.rs` |
| `fn get` | `src/storage.rs` |
| `fn put` | `src/storage.rs` |
| `fn delete` | `src/storage.rs` |
| `fn find` | `src/storage.rs` |
| `const CALL_FLAGS_ALL` | `src/syscalls.rs` |
| `fn contract_call` | `src/syscalls.rs` |
| `fn build_contract_call_stack` | `src/syscalls.rs` |
| `fn contract_call_with_flags` | `src/syscalls.rs` |
| `fn contract_create` | `src/syscalls.rs` |
| `fn contract_update` | `src/syscalls.rs` |
| `fn contract_get_call_flags` | `src/syscalls.rs` |
| `fn contract_create_standard_account` | `src/syscalls.rs` |
| `fn contract_create_multisig_account` | `src/syscalls.rs` |
| `fn contract_native_on_persist` | `src/syscalls.rs` |
| `fn contract_native_post_persist` | `src/syscalls.rs` |
| `fn runtime_notify` | `src/syscalls.rs` |
| `fn runtime_log` | `src/syscalls.rs` |
| `fn runtime_check_witness` | `src/syscalls.rs` |
| `fn runtime_get_notifications` | `src/syscalls.rs` |
| `fn runtime_burn_gas` | `src/syscalls.rs` |
| `fn runtime_load_script` | `src/syscalls.rs` |
| `fn runtime_platform` | `src/syscalls.rs` |
| `fn runtime_get_trigger` | `src/syscalls.rs` |
| `fn runtime_get_network` | `src/syscalls.rs` |
| `fn runtime_get_address_version` | `src/syscalls.rs` |
| `fn runtime_get_script_container` | `src/syscalls.rs` |
| `fn runtime_get_executing_script_hash` | `src/syscalls.rs` |
| `fn runtime_get_calling_script_hash` | `src/syscalls.rs` |
| `fn runtime_get_entry_script_hash` | `src/syscalls.rs` |
| `fn runtime_get_time` | `src/syscalls.rs` |
| `fn runtime_get_invocation_counter` | `src/syscalls.rs` |
| `fn runtime_gas_left` | `src/syscalls.rs` |
| `fn runtime_get_random` | `src/syscalls.rs` |
| `fn runtime_current_signers` | `src/syscalls.rs` |
| `fn crypto_verify_signature` | `src/syscalls.rs` |
| `fn crypto_check_multisig` | `src/syscalls.rs` |
| `fn iterator_next` | `src/syscalls.rs` |
| `fn iterator_value` | `src/syscalls.rs` |
| `struct Hash160` | `src/types.rs` |
| `struct Hash256` | `src/types.rs` |
| `struct PublicKey` | `src/types.rs` |
| `fn host_call` | `tests/native_wrappers_test.rs` |
| `fn host_call` | `tests/syscalls_shape_test.rs` |
| `fn host_call` | `tests/syscalls_test.rs` |

## Module and Re-Export Signals

| Signal |
| --- |
| `src/lib.rs: mod api_ids` |
| `src/lib.rs: mod codec` |
| `src/lib.rs: mod events` |
| `src/lib.rs: mod ffi` |
| `src/lib.rs: mod native` |
| `src/lib.rs: mod parser` |
| `src/lib.rs: mod signing` |
| `src/lib.rs: mod storage` |
| `src/lib.rs: mod syscalls` |
| `src/lib.rs: mod types` |
| `src/lib.rs: pub use codec::{     decode_bool_result, decode_bytes_result, decode_int_result, decode_string_result, encode_bytes,     encode_int_params, encode_string_params, }` |
| `src/lib.rs: pub use events::{     notify, notify_args, notify_int, notify_key_value, notify_string, notify_two_strings, }` |
| `src/lib.rs: pub use parser::{format_stack_value, parse_int_result, parse_stack_value, parse_string_result}` |
| `src/lib.rs: pub use signing::{check_witness, verify_multisig, verify_signature}` |
| `src/lib.rs: pub use storage::{delete, get, put}` |
| `src/lib.rs: pub use syscalls::{contract_call, contract_create, contract_update}` |
| `src/native/mod.rs: mod contract_management` |
| `src/native/mod.rs: mod crypto_lib` |
| `src/native/mod.rs: mod gas_token` |
| `src/native/mod.rs: mod ledger` |
| `src/native/mod.rs: mod neo_token` |
| `src/native/mod.rs: mod notary` |
| `src/native/mod.rs: mod oracle` |
| `src/native/mod.rs: mod policy` |
| `src/native/mod.rs: mod role_management` |
| `src/native/mod.rs: mod std_lib` |
| `src/native/mod.rs: mod treasury` |

## Test Evidence

| Test | File |
| --- | --- |
| `build_contract_call_stack_matches_bridge_shape` | `src/native/mod.rs` |
| `api_ids_match_interop_hashes` | `tests/api_ids_test.rs` |
| `gas_balance_of_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `neo_balance_of_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `policy_fee_wrapper_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `crypto_hash_wrapper_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `stdlib_encode_wrapper_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `ledger_current_index_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `role_management_wrapper_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `oracle_request_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `contract_management_wrapper_routes_through_contract_call` | `tests/native_wrappers_test.rs` |
| `build_contract_call_stack_matches_bridge_tail_order` | `tests/syscalls_shape_test.rs` |
| `build_contract_call_stack_preserves_custom_flags_and_empty_args` | `tests/syscalls_shape_test.rs` |
| `storage_get_returns_none` | `tests/syscalls_test.rs` |
| `storage_put_does_not_panic` | `tests/syscalls_test.rs` |
| `storage_delete_does_not_panic` | `tests/syscalls_test.rs` |
| `contract_call_returns_null` | `tests/syscalls_test.rs` |
| `contract_call_with_flags_returns_null` | `tests/syscalls_test.rs` |
| `runtime_notify_does_not_panic` | `tests/syscalls_test.rs` |
| `runtime_log_does_not_panic` | `tests/syscalls_test.rs` |
| `runtime_check_witness_returns_false` | `tests/syscalls_test.rs` |
| `crypto_verify_signature_returns_false` | `tests/syscalls_test.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-riscv-abi` | runtime |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.
2. Read `src/api_ids.rs`: implementation detail or helper module.
3. Read `src/syscalls.rs`: host syscall contract and dispatch boundary.
4. Read `src/native/std_lib.rs`: implementation detail or helper module.
5. Read `src/storage.rs`: implementation detail or helper module.
6. Read `src/native/neo_token.rs`: implementation detail or helper module.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Build artifacts, Validate metadata, Package deployment files.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
