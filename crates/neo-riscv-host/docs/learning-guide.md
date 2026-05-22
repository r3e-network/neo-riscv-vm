# neo-riscv-host Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Host runtime that executes PolkaVM guest modules, accounts gas, and bridges syscalls. |
| Inputs | PolkaVM module, execution context, host syscall provider |
| Responsibilities | Instantiate module, Charge opcodes, Marshal stack values, Return VM result |
| Outputs | execution result, gas report, host trace |
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
| `src/lib.rs` | crate root, public exports, and top-level documentation | 29 | 0 |
| `tests/runtime.rs` | external behavior or integration test | 0 | 156 |
| `src/runtime_cache.rs` | execution runtime, state transition, or gas behavior | 15 | 3 |
| `tests/fuzz_compatibility.rs` | external behavior or integration test | 0 | 47 |
| `src/bridge.rs` | bridge message, relay, or cross-chain boundary logic | 11 | 8 |
| `tests/parity.rs` | external behavior or integration test | 0 | 39 |
| `src/pricing.rs` | implementation detail or helper module | 6 | 12 |
| `src/ffi.rs` | implementation detail or helper module | 6 | 0 |
| `src/profiling.rs` | implementation detail or helper module | 5 | 0 |
| `tests/opcode_matrix/generator.rs` | external behavior or integration test | 2 | 1 |
| `tests/opcode_matrix/types.rs` | external behavior or integration test | 0 | 7 |
| `tests/opcode_matrix/arithmetic.rs` | external behavior or integration test | 0 | 6 |
| `tests/opcode_matrix/stack.rs` | external behavior or integration test | 0 | 6 |
| `tests/source_layout.rs` | external behavior or integration test | 0 | 5 |
| `tests/opcode_matrix/control.rs` | external behavior or integration test | 0 | 4 |
| `tests/opcode_matrix/regression.rs` | external behavior or integration test | 1 | 1 |
| `benches/benchmarks/arithmetic.rs` | implementation detail or helper module | 1 | 0 |
| `benches/benchmarks/codec.rs` | implementation detail or helper module | 1 | 0 |
| `benches/benchmarks/control_flow.rs` | implementation detail or helper module | 1 | 0 |
| `benches/benchmarks/overhead.rs` | implementation detail or helper module | 1 | 0 |
| `benches/benchmarks/stack_ops.rs` | implementation detail or helper module | 1 | 0 |
| `tests/csharp_contracts.rs` | external behavior or integration test | 0 | 2 |
| `tests/error_propagation.rs` | external behavior or integration test | 0 | 2 |
| `tests/gas_validation.rs` | external behavior or integration test | 0 | 2 |
| `tests/opcode_matrix/fuzzing.rs` | external behavior or integration test | 0 | 2 |
| `benches/benchmark_harness.rs` | implementation detail or helper module | 0 | 0 |
| `benches/benchmarks/mod.rs` | implementation detail or helper module | 0 | 0 |
| `benches/propagate_update_bench.rs` | implementation detail or helper module | 0 | 0 |
| `examples/profile_hotspot.rs` | runnable example or tutorial fixture | 0 | 0 |
| `tests/opcode_matrix/mod.rs` | external behavior or integration test | 0 | 0 |
| `tests/opcode_matrix_tests.rs` | external behavior or integration test | 0 | 0 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `fn bench` | `benches/benchmarks/arithmetic.rs` |
| `fn bench` | `benches/benchmarks/codec.rs` |
| `fn bench` | `benches/benchmarks/control_flow.rs` |
| `fn bench` | `benches/benchmarks/overhead.rs` |
| `fn bench` | `benches/benchmarks/stack_ops.rs` |
| `type GuestTrace` | `src/bridge.rs` |
| `fn register_host_functions` | `src/bridge.rs` |
| `struct ClosureHost` | `src/bridge.rs` |
| `fn new` | `src/bridge.rs` |
| `fn new_builtin` | `src/bridge.rs` |
| `fn read_guest_trace` | `src/bridge.rs` |
| `fn read_guest_panic` | `src/bridge.rs` |
| `fn read_guest_last_interpreter_ip` | `src/bridge.rs` |
| `fn read_guest_result_diag` | `src/bridge.rs` |
| `fn read_guest_debug` | `src/bridge.rs` |
| `fn read_pc_trace` | `src/bridge.rs` |
| `struct NativeHostResult` | `src/ffi.rs` |
| `type NativeHostCallback` | `src/ffi.rs` |
| `type NativeHostFreeCallback` | `src/ffi.rs` |
| `struct NativeExecutionResult` | `src/ffi.rs` |
| `struct NativeStackItem` | `src/ffi.rs` |
| `struct NativeIntegerExecutionResult` | `src/ffi.rs` |
| `fn set_last_fault_ip` | `src/lib.rs` |
| `fn reset_last_fault_ip` | `src/lib.rs` |
| `fn last_fault_ip` | `src/lib.rs` |
| `fn set_last_fault_locals` | `src/lib.rs` |
| `fn reset_last_native_fee_consumed_pico` | `src/lib.rs` |
| `fn set_last_native_fee_consumed_pico` | `src/lib.rs` |
| `fn last_native_fee_consumed_pico` | `src/lib.rs` |
| `fn read_last_fault_locals` | `src/lib.rs` |
| `struct PolkaVmRuntime` | `src/lib.rs` |
| `struct RuntimeContext` | `src/lib.rs` |
| `fn new` | `src/lib.rs` |
| `fn backend_kind` | `src/lib.rs` |
| `fn execute_script` | `src/lib.rs` |
| `fn execute_script_with_trigger` | `src/lib.rs` |
| `fn execute_script_with_context` | `src/lib.rs` |
| `fn execute_script_with_host_and_stack` | `src/lib.rs` |
| `fn debug_execute_script_with_host_and_stack` | `src/lib.rs` |
| `fn execute_script_with_host_and_stack_and_ip` | `src/lib.rs` |
| `fn execute_script_with_host_and_stack_and_ip_and_initializer` | `src/lib.rs` |
| `fn execute_script_with_host_and_stack_and_ip_with_result_limit` | `src/lib.rs` |
| `fn execute_script_with_host_and_stack_and_ip_and_initializer_with_result_limit` | `src/lib.rs` |
| `struct HostCallbackResult` | `src/lib.rs` |
| `fn charge_native_metered_instructions` | `src/lib.rs` |
| `fn native_call_error` | `src/lib.rs` |
| `fn execute_native_contract` | `src/lib.rs` |
| `fn execute_script_with_host` | `src/lib.rs` |
| `fn execute_native_contract_builtin` | `src/lib.rs` |
| `fn execute_native_contract_builtin_by_id` | `src/lib.rs` |
| `fn builtin_host_callback` | `src/lib.rs` |
| `const NEO_INSTRUCTION_CEILING` | `src/pricing.rs` |
| `fn check_instruction_ceiling` | `src/pricing.rs` |
| `fn charge_opcode` | `src/pricing.rs` |
| `fn native_instruction_limit` | `src/pricing.rs` |
| `fn charge_native_instructions` | `src/pricing.rs` |
| `fn opcode_price` | `src/pricing.rs` |
| `fn record_allocation` | `src/profiling.rs` |
| `fn record_deallocation` | `src/profiling.rs` |
| `fn get_peak_memory` | `src/profiling.rs` |
| `fn get_current_memory` | `src/profiling.rs` |
| `fn reset` | `src/profiling.rs` |
| `struct CachedExecutionInstance` | `src/runtime_cache.rs` |
| `fn module` | `src/runtime_cache.rs` |
| `fn instance_mut` | `src/runtime_cache.rs` |
| `struct CachedNativeExecutionInstance` | `src/runtime_cache.rs` |
| `fn module` | `src/runtime_cache.rs` |
| `fn instance_mut` | `src/runtime_cache.rs` |
| `fn ensure_runtime_ready` | `src/runtime_cache.rs` |
| `fn cached_module` | `src/runtime_cache.rs` |
| `fn cached_instance_pre` | `src/runtime_cache.rs` |
| `fn cached_execution_instance` | `src/runtime_cache.rs` |
| `fn compile_native_module` | `src/runtime_cache.rs` |
| `fn cached_native_execution_instance` | `src/runtime_cache.rs` |
| `fn module_cache_len` | `src/runtime_cache.rs` |
| `fn instance_pre_cache_len` | `src/runtime_cache.rs` |
| `fn execution_instance_pool_len` | `src/runtime_cache.rs` |
| `fn generate_opcode_tests` | `tests/opcode_matrix/generator.rs` |
| `struct OpcodeTest` | `tests/opcode_matrix/generator.rs` |
| `fn capture_failing_case` | `tests/opcode_matrix/regression.rs` |

## Module and Re-Export Signals

| Signal |
| --- |
| `benches/benchmark_harness.rs: mod benchmarks` |
| `benches/benchmarks/mod.rs: mod arithmetic` |
| `benches/benchmarks/mod.rs: mod codec` |
| `benches/benchmarks/mod.rs: mod control_flow` |
| `benches/benchmarks/mod.rs: mod overhead` |
| `benches/benchmarks/mod.rs: mod stack_ops` |
| `src/lib.rs: mod bridge` |
| `src/lib.rs: mod ffi` |
| `src/lib.rs: mod pricing` |
| `src/lib.rs: mod profiling` |
| `src/lib.rs: mod runtime_cache` |
| `src/lib.rs: pub use ffi::{     neo_riscv_execute_native_contract, neo_riscv_execute_native_contract_builtin,     neo_riscv_execute_native_contract_builtin_by_id,     neo_riscv_execute_native_contract_builtin_i64_by_id, neo_riscv_execute_script,     neo_riscv_execute_script_with_host, neo_riscv_execute_script_with_host_and_initializer,     neo_riscv_execute_script_with_host_and_initializer_and_result_limit,     neo_riscv_execute_script_with_host_and_result_limit, neo_riscv_free_execution_result,     NativeExecutionResult, NativeHostCallback, NativeHostFreeCallback, NativeHostResult,     NativeStackItem, }` |
| `src/lib.rs: pub use profiling::{get_current_memory, get_peak_memory, reset as reset_profiling}` |
| `tests/opcode_matrix/mod.rs: mod arithmetic` |
| `tests/opcode_matrix/mod.rs: mod control` |
| `tests/opcode_matrix/mod.rs: mod fuzzing` |
| `tests/opcode_matrix/mod.rs: mod generator` |
| `tests/opcode_matrix/mod.rs: mod regression` |
| `tests/opcode_matrix/mod.rs: mod stack` |
| `tests/opcode_matrix/mod.rs: mod types` |
| `tests/opcode_matrix_tests.rs: mod opcode_matrix` |

## Test Evidence

| Test | File |
| --- | --- |
| `get` | `src/bridge.rs` |
| `small_entry_round_trip_stays_in_small_cache` | `src/bridge.rs` |
| `small_inline_slots_can_hold_multiple_entries` | `src/bridge.rs` |
| `large_entry_falls_back_without_breaking_small_entries` | `src/bridge.rs` |
| `remove_clears_small_and_heap_entries` | `src/bridge.rs` |
| `get_promoting_rehydrates_hot_small_from_inline_entries` | `src/bridge.rs` |
| `insert_migrates_small_entries_without_returning_stale_values` | `src/bridge.rs` |
| `insert_migrates_inline_entries_to_heap_without_returning_stale_values` | `src/bridge.rs` |
| `opcode_price_push_opcodes` | `src/pricing.rs` |
| `opcode_price_flow_control` | `src/pricing.rs` |
| `opcode_price_expensive_opcodes` | `src/pricing.rs` |
| `opcode_price_unknown_defaults_to_max` | `src/pricing.rs` |
| `charge_opcode_deducts_gas` | `src/pricing.rs` |
| `charge_opcode_insufficient_gas_errors` | `src/pricing.rs` |
| `instruction_ceiling_permits_counts_below_cap` | `src/pricing.rs` |
| `instruction_ceiling_rejects_at_cap` | `src/pricing.rs` |
| `instruction_ceiling_rejects_above_cap` | `src/pricing.rs` |
| `charge_opcode_skips_when_fee_factor_zero` | `src/pricing.rs` |
| `native_instruction_limit_uses_fee_factor` | `src/pricing.rs` |
| `charge_native_instructions_reports_fee` | `src/pricing.rs` |
| `reuses_cached_module_for_same_aux_size` | `src/runtime_cache.rs` |
| `reuses_cached_instance_pre_for_same_aux_size` | `src/runtime_cache.rs` |
| `returns_execution_instances_to_pool` | `src/runtime_cache.rs` |
| `test_all_csharp_contracts_load` | `tests/csharp_contracts.rs` |
| `test_contract_assignment_executes` | `tests/csharp_contracts.rs` |
| `error_propagation_guest_to_host` | `tests/error_propagation.rs` |
| `error_propagation_result_types` | `tests/error_propagation.rs` |
| `fuzz_fast_codec_single_value` | `tests/fuzz_compatibility.rs` |
| `fuzz_fast_codec_multi_value` | `tests/fuzz_compatibility.rs` |
| `fuzz_fast_codec_decode_never_panics` | `tests/fuzz_compatibility.rs` |
| `fuzz_callback_codec_ok_roundtrip` | `tests/fuzz_compatibility.rs` |
| `fuzz_callback_codec_err_roundtrip` | `tests/fuzz_compatibility.rs` |
| `fuzz_callback_codec_decode_never_panics` | `tests/fuzz_compatibility.rs` |
| `fuzz_random_bytecode_no_panic` | `tests/fuzz_compatibility.rs` |
| `fuzz_pushint8` | `tests/fuzz_compatibility.rs` |
| `fuzz_pushint16` | `tests/fuzz_compatibility.rs` |
| `fuzz_pushint32` | `tests/fuzz_compatibility.rs` |
| `fuzz_pushint64` | `tests/fuzz_compatibility.rs` |
| `fuzz_pushdata1` | `tests/fuzz_compatibility.rs` |
| `fuzz_add` | `tests/fuzz_compatibility.rs` |
| `fuzz_sub` | `tests/fuzz_compatibility.rs` |
| `fuzz_mul` | `tests/fuzz_compatibility.rs` |
| `fuzz_div` | `tests/fuzz_compatibility.rs` |
| `fuzz_mod` | `tests/fuzz_compatibility.rs` |
| `fuzz_div_by_zero` | `tests/fuzz_compatibility.rs` |
| `fuzz_mod_by_zero` | `tests/fuzz_compatibility.rs` |
| `fuzz_negate` | `tests/fuzz_compatibility.rs` |
| `fuzz_abs` | `tests/fuzz_compatibility.rs` |
| `fuzz_inc` | `tests/fuzz_compatibility.rs` |
| `fuzz_dec` | `tests/fuzz_compatibility.rs` |
| `fuzz_lt` | `tests/fuzz_compatibility.rs` |
| `fuzz_le` | `tests/fuzz_compatibility.rs` |
| `fuzz_gt` | `tests/fuzz_compatibility.rs` |
| `fuzz_ge` | `tests/fuzz_compatibility.rs` |
| `fuzz_min` | `tests/fuzz_compatibility.rs` |
| `fuzz_max` | `tests/fuzz_compatibility.rs` |
| `fuzz_dup` | `tests/fuzz_compatibility.rs` |
| `fuzz_drop` | `tests/fuzz_compatibility.rs` |
| `fuzz_swap` | `tests/fuzz_compatibility.rs` |
| `fuzz_newarray` | `tests/fuzz_compatibility.rs` |
| `fuzz_pack` | `tests/fuzz_compatibility.rs` |
| `newarray_at_max_stack_size` | `tests/fuzz_compatibility.rs` |
| `newarray_exceeds_max_stack_size` | `tests/fuzz_compatibility.rs` |
| `newarray_t_exceeds_max_stack_size` | `tests/fuzz_compatibility.rs` |
| `newstruct_exceeds_max_stack_size` | `tests/fuzz_compatibility.rs` |
| `fuzz_bitwise_and` | `tests/fuzz_compatibility.rs` |
| `fuzz_bitwise_or` | `tests/fuzz_compatibility.rs` |
| `fuzz_bitwise_xor` | `tests/fuzz_compatibility.rs` |
| `fuzz_bitwise_no_panic` | `tests/fuzz_compatibility.rs` |
| `fuzz_not` | `tests/fuzz_compatibility.rs` |
| `fuzz_numequal` | `tests/fuzz_compatibility.rs` |
| `fuzz_numnotequal` | `tests/fuzz_compatibility.rs` |
| `fuzz_arithmetic_chain` | `tests/fuzz_compatibility.rs` |
| `fuzz_dup_add` | `tests/fuzz_compatibility.rs` |
| `runtime_initializes_for_gas_validation` | `tests/gas_validation.rs` |
| `opcode_fee_consumption_matches_configured_exec_fee_factor` | `tests/gas_validation.rs` |
| `add_basic` | `tests/opcode_matrix/arithmetic.rs` |
| `sub_basic` | `tests/opcode_matrix/arithmetic.rs` |
| `mul_basic` | `tests/opcode_matrix/arithmetic.rs` |
| `div_basic` | `tests/opcode_matrix/arithmetic.rs` |
| `div_by_zero_faults` | `tests/opcode_matrix/arithmetic.rs` |
| `mod_basic` | `tests/opcode_matrix/arithmetic.rs` |
| `jmpif_true_skips` | `tests/opcode_matrix/control.rs` |
| `jmpif_false_continues` | `tests/opcode_matrix/control.rs` |
| `jmpifnot_true_continues` | `tests/opcode_matrix/control.rs` |
| `ret_exits` | `tests/opcode_matrix/control.rs` |
| `pushint8_round_trips` | `tests/opcode_matrix/fuzzing.rs` |
| `pushdata1_round_trips` | `tests/opcode_matrix/fuzzing.rs` |
| `generates_256_tests` | `tests/opcode_matrix/generator.rs` |
| `captures_regression` | `tests/opcode_matrix/regression.rs` |
| `drop_empty_stack_faults` | `tests/opcode_matrix/stack.rs` |
| `dup_empty_stack_faults` | `tests/opcode_matrix/stack.rs` |
| `dup_basic` | `tests/opcode_matrix/stack.rs` |
| `swap_basic` | `tests/opcode_matrix/stack.rs` |
| `rot_basic` | `tests/opcode_matrix/stack.rs` |
| `reverse3_basic` | `tests/opcode_matrix/stack.rs` |
| `convert_int_to_bool_true` | `tests/opcode_matrix/types.rs` |
| `convert_int_to_bool_false` | `tests/opcode_matrix/types.rs` |
| `istype_integer` | `tests/opcode_matrix/types.rs` |
| `istype_boolean_false` | `tests/opcode_matrix/types.rs` |
| `newbuffer_at_limit` | `tests/opcode_matrix/types.rs` |
| `newbuffer_exceeds_limit` | `tests/opcode_matrix/types.rs` |
| `cat_exceeds_limit` | `tests/opcode_matrix/types.rs` |
| `parity_assignment` | `tests/parity.rs` |
| `parity_binary_expression` | `tests/parity.rs` |
| `parity_big_integer_pow` | `tests/parity.rs` |
| `parity_big_integer_sqrt` | `tests/parity.rs` |
| `parity_big_integer_parse_constant` | `tests/parity.rs` |
| `parity_boolean` | `tests/parity.rs` |
| `parity_default_values` | `tests/parity.rs` |
| `parity_types_basic` | `tests/parity.rs` |
| `parity_types_biginteger` | `tests/parity.rs` |
| `parity_foreach` | `tests/parity.rs` |
| `parity_goto` | `tests/parity.rs` |
| `parity_inc_dec` | `tests/parity.rs` |
| `parity_logical` | `tests/parity.rs` |
| `parity_recursion` | `tests/parity.rs` |
| `parity_checked_unchecked` | `tests/parity.rs` |
| `parity_returns` | `tests/parity.rs` |
| `parity_polymorphism` | `tests/parity.rs` |
| `parity_params` | `tests/parity.rs` |
| `parity_out_variables` | `tests/parity.rs` |
| `parity_pattern_matching` | `tests/parity.rs` |
| `parity_complex_assign` | `tests/parity.rs` |
| `parity_property` | `tests/parity.rs` |
| `parity_string` | `tests/parity.rs` |
| `parity_integer_operations` | `tests/parity.rs` |
| `parity_tuple` | `tests/parity.rs` |
| `parity_class_init` | `tests/parity.rs` |
| `parity_partial` | `tests/parity.rs` |
| `parity_partial_cross_file` | `tests/parity.rs` |
| `parity_inline` | `tests/parity.rs` |
| `parity_shift` | `tests/parity.rs` |
| `parity_delegate` | `tests/parity.rs` |
| `parity_index_or_range` | `tests/parity.rs` |
| `parity_member_access` | `tests/parity.rs` |
| `parity_postfix_unary` | `tests/parity.rs` |
| `parity_property_method` | `tests/parity.rs` |
| `parity_static_var` | `tests/parity.rs` |
| `parity_static_construct` | `tests/parity.rs` |
| `parity_static_class` | `tests/parity.rs` |
| `parity_bulk_zero_arg_methods` | `tests/parity.rs` |
| `creates_interpreter_backed_polkavm_runtime` | `tests/runtime.rs` |
| `executes_push1_ret_through_host_runtime` | `tests/runtime.rs` |
| `helper_append_mutates_consumed_caller_array_alias_through_host_runtime` | `tests/runtime.rs` |
| `size_matches_neovm_for_integer_and_boolean_values_through_host_runtime` | `tests/runtime.rs` |
| `size_faults_on_null_like_neovm_through_host_runtime` | `tests/runtime.rs` |
| `pickitem_reads_integer_payload_through_host_runtime` | `tests/runtime.rs` |
| `shr_accepts_big_integer_operand_through_host_runtime` | `tests/runtime.rs` |
| `wide_positive_integer_numeric_ops_work_through_host_runtime` | `tests/runtime.rs` |
| `empty_buffer_substr_reverses_through_host_runtime` | `tests/runtime.rs` |
| `setitem_updates_local_alias_extracted_from_parent_array_through_host_runtime` | `tests/runtime.rs` |
| `call_helper_updates_nested_map_argument_alias_through_host_runtime` | `tests/runtime.rs` |
| `executes_runtime_platform_syscall_through_host_runtime` | `tests/runtime.rs` |
| `executes_runtime_get_trigger_syscall_through_host_runtime` | `tests/runtime.rs` |
| `caller_catch_restores_locals_after_callee_throw_in_host_runtime` | `tests/runtime.rs` |
| `callt_null_result_can_flow_through_two_arg_helper_in_host_runtime` | `tests/runtime.rs` |
| `callt_null_result_can_round_trip_through_local_in_host_runtime` | `tests/runtime.rs` |
| `callt_null_result_preserves_caller_args_in_host_runtime` | `tests/runtime.rs` |
| `callt_null_result_can_flow_through_two_arg_helper_at_nonzero_ip_in_host_runtime` | `tests/runtime.rs` |
| `callt_block_like_struct_round_trips_directly_in_host_runtime` | `tests/runtime.rs` |
| `callt_block_like_struct_survives_local_pickitem_in_host_runtime` | `tests/runtime.rs` |
| `callt_block_like_struct_survives_local_pickitem_with_live_args_in_host_runtime` | `tests/runtime.rs` |
| `helper_entry_pickitem_on_block_like_struct_arg_in_host_runtime` | `tests/runtime.rs` |
| `tx_like_struct_hash_then_callt_signers_in_host_runtime` | `tests/runtime.rs` |
| `callt_transaction_helper_then_callt_signers_with_live_args_in_host_runtime` | `tests/runtime.rs` |
| `local_block_like_struct_then_helper_pickitem_in_host_runtime` | `tests/runtime.rs` |
| `executes_runtime_get_network_syscall_through_host_runtime` | `tests/runtime.rs` |
| `executes_runtime_gas_left_syscall_through_host_runtime` | `tests/runtime.rs` |
| `executes_platform_syscall_through_custom_host_callback` | `tests/runtime.rs` |
| `executes_null_stack_item_through_custom_host_callback` | `tests/runtime.rs` |
| `executes_array_stack_item_through_custom_host_callback` | `tests/runtime.rs` |
| `executes_struct_stack_item_through_custom_host_callback` | `tests/runtime.rs` |
| `notifications_like_result_round_trips_through_host_runtime` | `tests/runtime.rs` |
| `contract_state_like_result_round_trips_through_host_runtime` | `tests/runtime.rs` |
| `custom_host_callback_receives_input_stack_for_log_style_syscall` | `tests/runtime.rs` |
| `custom_host_callback_receives_current_instruction_pointer` | `tests/runtime.rs` |
| `executes_runtime_get_time_syscall_through_host_runtime` | `tests/runtime.rs` |
| `runtime_get_time_syscall_faults_without_timestamp` | `tests/runtime.rs` |
| `polkavm_execution_reports_opcode_fee_consumed` | `tests/runtime.rs` |
| `custom_host_callback_handles_large_bytestring_array_argument` | `tests/runtime.rs` |
| `custom_host_callback_handles_large_dynamic_call_shape_with_prior_stack_item` | `tests/runtime.rs` |
| `large_dynamic_call_host_error_surfaces_without_trap` | `tests/runtime.rs` |
| `large_dynamic_call_wrapper_host_error_surfaces_without_trap` | `tests/runtime.rs` |
| `large_dynamic_call_wrapper_host_error_surfaces_without_trap_with_fee_accounting` | `tests/runtime.rs` |
| `dynamic_call_wrapper_executes_from_nonempty_stack_with_large_argument` | `tests/runtime.rs` |
| `dynamic_call_wrapper_executes_after_dropping_large_argument_array` | `tests/runtime.rs` |
| `large_dynamic_call_executes_after_small_prior_syscall` | `tests/runtime.rs` |
| `second_large_dynamic_call_executes_if_first_result_is_dropped` | `tests/runtime.rs` |
| `contract_call_bool_result_survives_heap_backed_locals_after_syscall` | `tests/runtime.rs` |
| `contract_call_bool_result_survives_multisig_like_heap_locals_after_syscall` | `tests/runtime.rs` |
| `contract_call_bool_result_survives_multisig_like_initial_stack_across_contexts` | `tests/runtime.rs` |
| `contract_call_bool_result_survives_multisig_like_scriptcontainer_path` | `tests/runtime.rs` |
| `contract_call_bool_result_survives_multisig_like_full_transaction_scriptcontainer_path` | `tests/runtime.rs` |
| `full_transaction_scriptcontainer_path_preserves_integer_local_without_following_host_call` | `tests/runtime.rs` |
| `contract_call_bool_result_after_full_transaction_scriptcontainer_survives_single_iteration` | `tests/runtime.rs` |
| `consecutive_large_dynamic_calls_can_return_single_final_integer` | `tests/runtime.rs` |
| `custom_host_callback_handles_interop_array_argument` | `tests/runtime.rs` |
| `custom_host_callback_handles_packed_interop_results_across_multiple_syscalls` | `tests/runtime.rs` |
| `custom_host_callback_preserves_interop_result_between_syscalls` | `tests/runtime.rs` |
| `custom_host_callback_preserves_integer_result_between_syscalls` | `tests/runtime.rs` |
| `custom_host_callback_preserves_integer_then_bytestring_between_syscalls` | `tests/runtime.rs` |
| `local_storage_round_trip_survives_delete_and_following_get` | `tests/runtime.rs` |
| `local_storage_get_result_survives_pushdata_before_delete` | `tests/runtime.rs` |
| `local_storage_get_result_survives_delete_before_next_get` | `tests/runtime.rs` |
| `bytestring_result_survives_pushdata_before_next_syscall` | `tests/runtime.rs` |
| `retained_bytestring_survives_one_arg_syscall_with_no_results` | `tests/runtime.rs` |
| `retained_bytestring_survives_no_result_then_null_result_syscalls` | `tests/runtime.rs` |
| `retained_bytestring_and_null_can_be_observed_before_ret` | `tests/runtime.rs` |
| `helper_syscalls_preserve_multiple_arguments_and_order_in_host_runtime` | `tests/runtime.rs` |
| `helper_syscalls_preserve_large_arguments_before_callt_in_host_runtime` | `tests/runtime.rs` |
| `large_proxy_update_arguments_survive_helper_calls_before_callt_in_host_runtime` | `tests/runtime.rs` |
| `large_proxy_update_arguments_survive_initializer_before_callt_in_host_runtime` | `tests/runtime.rs` |
| `initializer_host_call_preserves_large_method_arguments_in_host_runtime` | `tests/runtime.rs` |
| `large_proxy_update_arguments_survive_pushdata_pack_and_contract_call_in_host_runtime` | `tests/runtime.rs` |
| `custom_host_callback_can_return_two_integers` | `tests/runtime.rs` |
| `custom_host_callback_preserves_initial_two_integer_response_between_syscalls` | `tests/runtime.rs` |
| `custom_host_callback_state_persists_across_multiple_syscalls` | `tests/runtime.rs` |
| `custom_host_callback_can_return_fresh_two_item_second_response` | `tests/runtime.rs` |
| `debug_success_trace_for_two_item_second_response` | `tests/runtime.rs` |
| `custom_host_callback_exposes_second_response_to_next_syscall` | `tests/runtime.rs` |
| `custom_host_callback_exposes_fresh_second_response_to_next_syscall` | `tests/runtime.rs` |
| `custom_host_callback_handles_contract_call_shape_with_interop_array_argument` | `tests/runtime.rs` |
| `unsupported_opcode_returns_fault_state` | `tests/runtime.rs` |
| `and_on_booleans_through_polkavm_returns_integer` | `tests/runtime.rs` |
| `handles_multiple_syscalls_with_pack_and_interop` | `tests/runtime.rs` |
| `large_dynamic_call_with_bytestring_argument_can_return_interop` | `tests/runtime.rs` |
| `result_limit_trims_internal_halt_stack_before_abi_conversion` | `tests/runtime.rs` |
| `ffi_host_callback_errors_fault_without_trapping` | `tests/runtime.rs` |
| `ffi_callt_null_result_can_flow_through_two_arg_helper` | `tests/runtime.rs` |
| `ffi_callt_block_like_struct_survives_local_and_helper_pickitem` | `tests/runtime.rs` |
| `ffi_tx_like_struct_hash_then_callt_signers` | `tests/runtime.rs` |
| `ffi_callt_transaction_helper_then_callt_signers_with_live_args` | `tests/runtime.rs` |
| `ffi_large_dynamic_call_host_error_surfaces_without_trapping` | `tests/runtime.rs` |
| `ffi_large_dynamic_call_wrapper_host_error_surfaces_without_trapping` | `tests/runtime.rs` |
| `ffi_mixed_integer_and_bytestring_results_round_trip` | `tests/runtime.rs` |
| `storage_context_token_round_trips_across_syscalls_in_host_path` | `tests/runtime.rs` |
| `storage_context_token_round_trips_across_syscalls_in_ffi_path` | `tests/runtime.rs` |
| `popitem_removes_last_array_element` | `tests/runtime.rs` |
| `popitem_removes_last_struct_element` | `tests/runtime.rs` |
| `callt_invokes_host_callback` | `tests/runtime.rs` |
| `callt_array_result_round_trips_through_locals_and_pickitem_in_host_runtime` | `tests/runtime.rs` |
| `callt_array_result_round_trips_with_live_args_in_host_runtime` | `tests/runtime.rs` |
| `callt_string_result_can_setitem_into_live_array_after_cat_in_host_runtime` | `tests/runtime.rs` |
| `contract_call_bytes_result_can_be_normalized_and_setitem_into_map` | `tests/runtime.rs` |
| `ffi_callt_string_result_can_setitem_into_live_array_after_cat` | `tests/runtime.rs` |
| `ffi_callt_retains_consumed_mutations_before_later_setitem` | `tests/runtime.rs` |
| `callt_array_result_round_trips_with_live_args_in_ffi_host_runtime` | `tests/runtime.rs` |
| `callt_array_result_survives_prior_syscall_with_live_args_in_host_runtime` | `tests/runtime.rs` |
| `callt_array_result_survives_prior_hash_syscall_with_live_args_in_host_runtime` | `tests/runtime.rs` |
| `oracle_on_response_success_path_executes_in_host_runtime` | `tests/runtime.rs` |
| `oracle_on_response_success_path_executes_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_static_fields_in_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_hash_static_fields_in_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_allows_deploy_to_compare_hash_static_fields_in_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_continues_to_later_target_method_in_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_target_method_arguments_in_host_runtime` | `tests/runtime.rs` |
| `initializer_static_field_prefix_survives_repeated_storage_puts_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_storage_get_then_call_l_survives_alias_mutation_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_target_method_arguments_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_ldarg0_for_checkwitness_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_large_ten_argument_stack_in_ffi_host_runtime` | `tests/runtime.rs` |
| `initializer_entrypoint_preserves_nested_array_arguments_in_ffi_host_runtime` | `tests/runtime.rs` |
| `attribute_test_path_executes_in_ffi_host_runtime` | `tests/runtime.rs` |
| `attribute_owner_constructor_helper_updates_array_in_host_runtime` | `tests/runtime.rs` |
| `attribute_owner_constructor_helper_updates_picked_array_in_host_runtime` | `tests/runtime.rs` |
| `attribute_test_path_executes_in_host_runtime` | `tests/runtime.rs` |
| `attribute_test_path_survives_prior_noop_call_in_host_runtime` | `tests/runtime.rs` |
| `attribute_test_path_survives_prior_initsslot_call_in_host_runtime` | `tests/runtime.rs` |
| `attribute_test_path_executes_via_minimal_initialize_wrapper_in_host_runtime` | `tests/runtime.rs` |
| `callt_array_result_survives_prior_call_with_live_stack_in_host_runtime` | `tests/runtime.rs` |
| `test_try_catch_syscall_exception` | `tests/runtime.rs` |
| `test_try_catch_throw_simple` | `tests/runtime.rs` |
| `try_catch_catches_pickitem_type_error_in_host_runtime` | `tests/runtime.rs` |
| `initializer_then_target_catches_pickitem_type_error_in_host_runtime` | `tests/runtime.rs` |
| `gas_exhaustion_through_polkavm` | `tests/runtime.rs` |
| `ffi_script_gas_exhaustion_reports_consumed_fee` | `tests/runtime.rs` |
| `pointer_type_through_host_callback` | `tests/runtime.rs` |
| `biginteger_through_host_callback` | `tests/runtime.rs` |
| `pow_accepts_big_integer_result_through_host_runtime` | `tests/runtime.rs` |
| `block_78538_contract_deploy_does_not_trap` | `tests/runtime.rs` |
| `block_78538_ffi_path_does_not_trap` | `tests/runtime.rs` |
| `test_csharp_compiled_native_contract` | `tests/runtime.rs` |
| `test_native_contract_missing_check_witness_unsafe_update` | `tests/runtime.rs` |
| `test_native_contract_echo_args` | `tests/runtime.rs` |
| `test_native_contract_echo_after_bridge` | `tests/runtime.rs` |
| `test_native_contract_echo_after_bridge_with_local` | `tests/runtime.rs` |
| `test_native_contract_echo_after_2_bridges` | `tests/runtime.rs` |
| `test_native_contract_echo_after_2_bridges_local` | `tests/runtime.rs` |
| `test_native_contract_two_bridges_no_host` | `tests/runtime.rs` |
| `test_native_contract_missing_check_witness_safe_update` | `tests/runtime.rs` |
| `test_native_contract_checkwitness_simple` | `tests/runtime.rs` |
| `test_native_contract_checkwitness_init3` | `tests/runtime.rs` |
| `test_native_contract_safe_update_minimal_args` | `tests/runtime.rs` |
| `test_native_contract_unsafe_update_two_syscalls` | `tests/runtime.rs` |
| `test_native_contract_checkwitness_with_assert` | `tests/runtime.rs` |
| `test_native_contract_checkwitness_then_put_hardcoded` | `tests/runtime.rs` |
| `test_native_contract_checkwitness_getcontext_put` | `tests/runtime.rs` |
| `host_opcode_pricing_uses_shared_opcode_enum` | `tests/source_layout.rs` |
| `host_bridge_uses_shared_stack_value_codec_tags` | `tests/source_layout.rs` |
| `fuzz_opcode_generation_uses_shared_opcode_enum` | `tests/source_layout.rs` |
| `opcode_matrix_names_use_shared_opcode_enum` | `tests/source_layout.rs` |
| `fuzz_targets_use_shared_opcode_enum` | `tests/source_layout.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-riscv-abi` | runtime |
| `neo-riscv-guest` | runtime |
| `polkavm` | runtime |
| `postcard` | runtime |
| `tracing` | runtime |
| `criterion` | test |
| `proptest` | test |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.
2. Read `tests/runtime.rs`: external behavior or integration test.
3. Read `src/runtime_cache.rs`: execution runtime, state transition, or gas behavior.
4. Read `tests/fuzz_compatibility.rs`: external behavior or integration test.
5. Read `src/bridge.rs`: bridge message, relay, or cross-chain boundary logic.
6. Read `tests/parity.rs`: external behavior or integration test.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Instantiate module, Charge opcodes, Marshal stack values, Return VM result.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
