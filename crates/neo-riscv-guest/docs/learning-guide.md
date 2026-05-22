# neo-riscv-guest Source-Level Learning Guide

This guide is generated from the crate's actual `Cargo.toml`, Rust source files, public symbols, and test functions. It is meant to help a reader understand what this crate owns before reading implementation details.

## What This Crate Is

| Topic | Detail |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Guest-side facade and contract runtime glue for NeoVM2/RISC-V contracts. |
| Inputs | contract bytecode, ABI stack, syscall stubs |
| Responsibilities | Call shared VM runtime, Expose no_std contract APIs, Bridge syscalls |
| Outputs | guest result, syscall request, stack mutation |
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
| `src/lib.rs` | crate root, public exports, and top-level documentation | 0 | 0 |
| `tests/interpreter.rs` | external behavior or integration test | 0 | 217 |
| `tests/contracts.rs` | external behavior or integration test | 0 | 49 |
| `src/contract_rt/mod.rs` | implementation detail or helper module | 9 | 3 |
| `tests/contract_rt_context.rs` | external behavior or integration test | 0 | 14 |
| `tests/contract_rt_layering.rs` | external behavior or integration test | 0 | 3 |
| `tests/rc_compat_test.rs` | external behavior or integration test | 0 | 3 |
| `tests/shared_vm_dependency.rs` | external behavior or integration test | 0 | 3 |
| `tests/contract_rt.rs` | external behavior or integration test | 0 | 2 |
| `tests/workspace_layering.rs` | external behavior or integration test | 0 | 1 |
| `src/contract_rt/mem_intrinsics.rs` | implementation detail or helper module | 0 | 0 |
| `src/contract_rt/stack_value.rs` | implementation detail or helper module | 0 | 0 |

## Public API Surface

| Symbol | File |
| --- | --- |
| `type SyscallBridgeFn` | `src/contract_rt/mod.rs` |
| `fn set_syscall_bridge` | `src/contract_rt/mod.rs` |
| `struct Context` | `src/contract_rt/mod.rs` |
| `fn from_abi_stack` | `src/contract_rt/mod.rs` |
| `fn into_vm_context` | `src/contract_rt/mod.rs` |
| `fn to_execution_result` | `src/contract_rt/mod.rs` |
| `fn syscall` | `src/contract_rt/mod.rs` |
| `fn call_token` | `src/contract_rt/mod.rs` |
| `fn calla` | `src/contract_rt/mod.rs` |

## Module and Re-Export Signals

| Signal |
| --- |
| `src/contract_rt/mod.rs: mod stack_value` |
| `src/contract_rt/mod.rs: mod mem_intrinsics` |
| `src/contract_rt/mod.rs: pub use stack_value::StackValue` |
| `src/contract_rt/stack_value.rs: pub use neo_riscv_abi::{     byte_sequence_bytes, byte_sequence_len, concat_byte_sequences, default_value_for_type_tag,     new_array_default_value_for_type_tag, normalize_stack_item_type_tag, slice_byte_sequence,     StackValue, TAG_ARRAY, TAG_BIG_INTEGER, TAG_BOOLEAN, TAG_BUFFER, TAG_BYTESTRING, TAG_INTEGER,     TAG_INTEROP, TAG_ITERATOR, TAG_MAP, TAG_NULL, TAG_POINTER, TAG_STRUCT, }` |
| `src/lib.rs: mod contract_rt` |
| `src/lib.rs: pub use neo_riscv_abi::semantics` |
| `src/lib.rs: pub use neo_vm_rs::{     fast_codec, interpret, interpret_with_stack_and_syscalls, interpret_with_stack_and_syscalls_at,     interpret_with_stack_and_syscalls_at_with_initializer,     interpret_with_stack_and_syscalls_at_with_initializer_and_result_limit,     interpret_with_stack_and_syscalls_at_with_result_limit, interpret_with_syscalls,     last_interpreter_ip, last_result_limit, last_result_stack_len, last_result_stage, BackendKind,     ExecutionResult, StackValue, SyscallProvider, VmState, CALLT_MARKER, CALLT_MARKER_HI,     INITIALIZER_COMPLETE_MARKER, }` |

## Test Evidence

| Test | File |
| --- | --- |
| `from_abi_stack_roundtrip` | `src/contract_rt/mod.rs` |
| `shared_runtime_ops_execute_against_riscv_context` | `src/contract_rt/mod.rs` |
| `riscv_syscall_stubs_fault_through_shared_vm_context` | `src/contract_rt/mod.rs` |
| `contract_runtime_context_executes_shared_vm_runtime_ops` | `tests/contract_rt.rs` |
| `contract_runtime_syscall_stubs_fault_through_shared_vm_context` | `tests/contract_rt.rs` |
| `push_pop_integer` | `tests/contract_rt_context.rs` |
| `init_slot_loads_args_from_stack` | `tests/contract_rt_context.rs` |
| `add_integers` | `tests/contract_rt_context.rs` |
| `sub_integers` | `tests/contract_rt_context.rs` |
| `mul_integers` | `tests/contract_rt_context.rs` |
| `equal_integers` | `tests/contract_rt_context.rs` |
| `not_equal_integers` | `tests/contract_rt_context.rs` |
| `local_variable_store_load` | `tests/contract_rt_context.rs` |
| `dup_and_swap` | `tests/contract_rt_context.rs` |
| `push_null` | `tests/contract_rt_context.rs` |
| `push_bool_true_and_false` | `tests/contract_rt_context.rs` |
| `static_fields_store_load` | `tests/contract_rt_context.rs` |
| `static_fields_reject_uninitialized_or_out_of_range_access` | `tests/contract_rt_context.rs` |
| `push_bytes` | `tests/contract_rt_context.rs` |
| `contract_runtime_does_not_reintroduce_opcode_adapters` | `tests/contract_rt_layering.rs` |
| `contract_runtime_does_not_define_private_byte_opcode_helpers` | `tests/contract_rt_layering.rs` |
| `contract_runtime_does_not_keep_empty_future_memory_placeholders` | `tests/contract_rt_layering.rs` |
| `token_balance_subtraction` | `tests/contracts.rs` |
| `token_balance_addition` | `tests/contracts.rs` |
| `token_transfer_full_sequence` | `tests/contracts.rs` |
| `token_balance_negative_underflow` | `tests/contracts.rs` |
| `runtime_platform_syscall` | `tests/contracts.rs` |
| `runtime_gas_left_syscall` | `tests/contracts.rs` |
| `double_syscall_contract` | `tests/contracts.rs` |
| `conditional_branch_if_true` | `tests/contracts.rs` |
| `conditional_branch_if_false` | `tests/contracts.rs` |
| `comparison_less_than` | `tests/contracts.rs` |
| `comparison_greater_than` | `tests/contracts.rs` |
| `comparison_equal` | `tests/contracts.rs` |
| `comparison_not_equal` | `tests/contracts.rs` |
| `comparison_within_range` | `tests/contracts.rs` |
| `comparison_within_range_false` | `tests/contracts.rs` |
| `contract_dispatch_pattern` | `tests/contracts.rs` |
| `array_creation_and_setitem` | `tests/contracts.rs` |
| `array_size_check` | `tests/contracts.rs` |
| `array_pack_order` | `tests/contracts.rs` |
| `array_multiple_setitems` | `tests/contracts.rs` |
| `map_set_and_get` | `tests/contracts.rs` |
| `map_multiple_entries` | `tests/contracts.rs` |
| `map_keys_operation` | `tests/contracts.rs` |
| `nested_array_operations` | `tests/contracts.rs` |
| `modular_exponentiation` | `tests/contracts.rs` |
| `square_root` | `tests/contracts.rs` |
| `min_max_operations` | `tests/contracts.rs` |
| `complex_arithmetic_expression` | `tests/contracts.rs` |
| `string_concatenation` | `tests/contracts.rs` |
| `string_substring` | `tests/contracts.rs` |
| `string_left` | `tests/contracts.rs` |
| `string_right` | `tests/contracts.rs` |
| `bitwise_and` | `tests/contracts.rs` |
| `bitwise_or` | `tests/contracts.rs` |
| `bitwise_xor` | `tests/contracts.rs` |
| `try_catch_catches_throw` | `tests/contracts.rs` |
| `try_finally_executes` | `tests/contracts.rs` |
| `convert_integer_to_boolean` | `tests/contracts.rs` |
| `convert_integer_to_bytes` | `tests/contracts.rs` |
| `abs_negate_operations` | `tests/contracts.rs` |
| `increment_decrement` | `tests/contracts.rs` |
| `sign_detection` | `tests/contracts.rs` |
| `boolean_logic_operations` | `tests/contracts.rs` |
| `zero_value_check` | `tests/contracts.rs` |
| `struct_pack_and_pickitem` | `tests/contracts.rs` |
| `reverse_items_on_array` | `tests/contracts.rs` |
| `clearitems_on_array` | `tests/contracts.rs` |
| `address_length_validation` | `tests/contracts.rs` |
| `address_length_mismatch` | `tests/contracts.rs` |
| `executes_push1_ret_script` | `tests/interpreter.rs` |
| `executes_integer_addition_script` | `tests/interpreter.rs` |
| `executes_platform_syscall_with_host_provider` | `tests/interpreter.rs` |
| `executes_pushdata1_bytestring_script` | `tests/interpreter.rs` |
| `executes_pushnull_and_newarray0_script` | `tests/interpreter.rs` |
| `duplicates_bytestring_with_dup` | `tests/interpreter.rs` |
| `packs_items_into_array` | `tests/interpreter.rs` |
| `creates_null_filled_array` | `tests/interpreter.rs` |
| `helper_append_mutates_consumed_caller_array_alias` | `tests/interpreter.rs` |
| `gets_collection_size` | `tests/interpreter.rs` |
| `size_matches_neovm_for_integer_and_boolean_values` | `tests/interpreter.rs` |
| `size_faults_on_null_like_neovm` | `tests/interpreter.rs` |
| `picks_array_item_by_index` | `tests/interpreter.rs` |
| `creates_array_in_script_builder_order` | `tests/interpreter.rs` |
| `creates_struct_in_script_builder_order` | `tests/interpreter.rs` |
| `creates_map_in_script_builder_order` | `tests/interpreter.rs` |
| `creates_empty_struct_and_map` | `tests/interpreter.rs` |
| `asserts_true_boolean` | `tests/interpreter.rs` |
| `executes_script_with_initial_stack` | `tests/interpreter.rs` |
| `halts_when_script_ends_without_explicit_ret` | `tests/interpreter.rs` |
| `executes_script_from_nonzero_offset` | `tests/interpreter.rs` |
| `call_helper_syscall_preserves_byte_string_arguments` | `tests/interpreter.rs` |
| `call_helper_syscall_preserves_multiple_arguments_and_order` | `tests/interpreter.rs` |
| `executes_pushint8_and_pushint16` | `tests/interpreter.rs` |
| `executes_pushint32` | `tests/interpreter.rs` |
| `executes_pushtrue_and_pushfalse` | `tests/interpreter.rs` |
| `executes_pushint128_as_big_integer` | `tests/interpreter.rs` |
| `i128_arithmetic_results_feed_unary_numeric_ops` | `tests/interpreter.rs` |
| `sign_accepts_positive_integer_wider_than_i128` | `tests/interpreter.rs` |
| `numeric_ops_accept_positive_integer_wider_than_i128` | `tests/interpreter.rs` |
| `numeric_jump_accepts_positive_integer_wider_than_i128` | `tests/interpreter.rs` |
| `i128_arithmetic_results_feed_min_max_and_within` | `tests/interpreter.rs` |
| `passes_current_instruction_pointer_to_syscall_provider` | `tests/interpreter.rs` |
| `retained_bytestring_survives_no_result_then_null_result_syscalls_in_interpreter` | `tests/interpreter.rs` |
| `local_storage_round_trip_survives_delete_and_following_get_in_interpreter` | `tests/interpreter.rs` |
| `concatenates_integer_as_bytestring_preserving_sign_bit` | `tests/interpreter.rs` |
| `concatenates_negative_integer_as_bytestring` | `tests/interpreter.rs` |
| `invert_empty_bytestring_matches_neovm_integer_semantics` | `tests/interpreter.rs` |
| `empty_buffer_is_truthy_for_boolean_opcodes` | `tests/interpreter.rs` |
| `executes_pushint128_positive_preserves_sign_bit` | `tests/interpreter.rs` |
| `executes_pushint128_negative_trims_correctly` | `tests/interpreter.rs` |
| `executes_jmp_forward` | `tests/interpreter.rs` |
| `executes_drop_and_swap` | `tests/interpreter.rs` |
| `executes_sub_and_inc` | `tests/interpreter.rs` |
| `executes_sub_and_inc_on_integer_compatible_bytestrings` | `tests/interpreter.rs` |
| `executes_numequal_and_ge` | `tests/interpreter.rs` |
| `executes_numequal_on_bytestrings` | `tests/interpreter.rs` |
| `executes_numequal_on_bytestring_and_integer` | `tests/interpreter.rs` |
| `executes_numequal_on_big_integer_and_integer` | `tests/interpreter.rs` |
| `executes_numnotequal_on_bytestrings` | `tests/interpreter.rs` |
| `ge_with_null_operand_returns_false` | `tests/interpreter.rs` |
| `comparison_with_null_and_buffer_returns_false_like_neovm` | `tests/interpreter.rs` |
| `executes_lt_le_and_gt` | `tests/interpreter.rs` |
| `executes_negate_and_sign` | `tests/interpreter.rs` |
| `sign_accepts_bytestring_input` | `tests/interpreter.rs` |
| `executes_modmul` | `tests/interpreter.rs` |
| `modmul_accepts_i128_operands_and_modulus_like_neovm` | `tests/interpreter.rs` |
| `executes_pow` | `tests/interpreter.rs` |
| `pow_accepts_big_integer_result` | `tests/interpreter.rs` |
| `executes_sqrt` | `tests/interpreter.rs` |
| `executes_shl_and_shr` | `tests/interpreter.rs` |
| `shr_preserves_bytestring_type` | `tests/interpreter.rs` |
| `shr_accepts_big_integer_operand` | `tests/interpreter.rs` |
| `shr_converts_wide_bytestring_result_to_integer_like_neovm` | `tests/interpreter.rs` |
| `zero_shift_preserves_null_and_boolean_like_neovm` | `tests/interpreter.rs` |
| `negative_shift_count_faults_like_neovm` | `tests/interpreter.rs` |
| `executes_newarray_t` | `tests/interpreter.rs` |
| `executes_haskey_on_buffer` | `tests/interpreter.rs` |
| `executes_and_on_bytestrings` | `tests/interpreter.rs` |
| `bitwise_ops_accept_mixed_primitive_numeric_values` | `tests/interpreter.rs` |
| `executes_keys_on_map` | `tests/interpreter.rs` |
| `setitem_updates_static_field_alias` | `tests/interpreter.rs` |
| `setitem_updates_local_alias_extracted_from_parent_array` | `tests/interpreter.rs` |
| `call_helper_updates_nested_map_argument_alias` | `tests/interpreter.rs` |
| `append_updates_static_field_alias` | `tests/interpreter.rs` |
| `initializer_static_fields_are_visible_to_target_method` | `tests/interpreter.rs` |
| `ldsfld_without_static_slot_faults` | `tests/interpreter.rs` |
| `ldsfld_out_of_range_faults` | `tests/interpreter.rs` |
| `stsfld_out_of_range_faults` | `tests/interpreter.rs` |
| `clearitems_updates_static_field_alias` | `tests/interpreter.rs` |
| `reverseitems_updates_buffer_alias` | `tests/interpreter.rs` |
| `remove_updates_static_field_alias` | `tests/interpreter.rs` |
| `unpack_array_preserves_stack_order` | `tests/interpreter.rs` |
| `unpack_map_preserves_key_value_order` | `tests/interpreter.rs` |
| `executes_modpow_and_mod_inverse` | `tests/interpreter.rs` |
| `modpow_zero_exponent_is_reduced_by_modulus_like_neovm` | `tests/interpreter.rs` |
| `modpow_zero_exponent_with_negative_modulus_returns_positive_one_like_neovm` | `tests/interpreter.rs` |
| `modpow_preserves_signed_remainder_for_negative_base_like_neovm` | `tests/interpreter.rs` |
| `modpow_accepts_i128_operands_and_modulus_like_neovm` | `tests/interpreter.rs` |
| `modpow_zero_inverse_faults_like_neovm` | `tests/interpreter.rs` |
| `modpow_invalid_inverse_inputs_fault_like_neovm` | `tests/interpreter.rs` |
| `executes_not` | `tests/interpreter.rs` |
| `executes_not_on_empty_array_and_struct` | `tests/interpreter.rs` |
| `executes_initslot_stloc_ldloc` | `tests/interpreter.rs` |
| `executes_or_operator` | `tests/interpreter.rs` |
| `executes_or_on_integers` | `tests/interpreter.rs` |
| `executes_left_on_bytestring` | `tests/interpreter.rs` |
| `executes_nop` | `tests/interpreter.rs` |
| `abort_faults_with_message` | `tests/interpreter.rs` |
| `assert_false_faults_with_message` | `tests/interpreter.rs` |
| `executes_jmpif_conditional` | `tests/interpreter.rs` |
| `executes_pushint64` | `tests/interpreter.rs` |
| `executes_depth` | `tests/interpreter.rs` |
| `executes_and_on_integers` | `tests/interpreter.rs` |
| `executes_and_on_booleans` | `tests/interpreter.rs` |
| `executes_xor_on_integers` | `tests/interpreter.rs` |
| `executes_xor_on_booleans` | `tests/interpreter.rs` |
| `executes_invert_on_integer` | `tests/interpreter.rs` |
| `executes_invert_on_boolean` | `tests/interpreter.rs` |
| `invert_faults_on_null_like_neovm` | `tests/interpreter.rs` |
| `numeric_ops_fault_on_null_like_neovm` | `tests/interpreter.rs` |
| `numeric_ops_fault_on_buffer_like_neovm` | `tests/interpreter.rs` |
| `integer_comparison_faults_on_buffer_like_neovm` | `tests/interpreter.rs` |
| `executes_equal_on_same_integers` | `tests/interpreter.rs` |
| `executes_equal_on_different_integers` | `tests/interpreter.rs` |
| `executes_equal_on_same_array_reference` | `tests/interpreter.rs` |
| `executes_equal_on_distinct_arrays` | `tests/interpreter.rs` |
| `executes_equal_on_distinct_structs_with_equal_content` | `tests/interpreter.rs` |
| `executes_equal_on_same_map_reference` | `tests/interpreter.rs` |
| `executes_equal_on_distinct_maps` | `tests/interpreter.rs` |
| `executes_equal_on_same_buffer_reference` | `tests/interpreter.rs` |
| `executes_equal_on_distinct_buffers_with_equal_content` | `tests/interpreter.rs` |
| `executes_notequal` | `tests/interpreter.rs` |
| `executes_values_on_map` | `tests/interpreter.rs` |
| `packs_interop_results_across_multiple_syscalls` | `tests/interpreter.rs` |
| `preserves_integer_results_across_multiple_syscalls` | `tests/interpreter.rs` |
| `executes_two_consecutive_large_dynamic_calls` | `tests/interpreter.rs` |
| `unsupported_opcode_returns_error` | `tests/interpreter.rs` |
| `and_on_booleans_returns_integer` | `tests/interpreter.rs` |
| `stack_overflow_at_2048_items` | `tests/interpreter.rs` |
| `newbuffer_rejects_over_max_item_size` | `tests/interpreter.rs` |
| `newbuffer_rejects_i256_length_over_i64_range` | `tests/interpreter.rs` |
| `empty_script_halts_immediately` | `tests/interpreter.rs` |
| `jmpifnot_jumps_when_false` | `tests/interpreter.rs` |
| `jmpeq_jumps_when_equal` | `tests/interpreter.rs` |
| `jmpne_jumps_when_not_equal` | `tests/interpreter.rs` |
| `call_and_ret_round_trip` | `tests/interpreter.rs` |
| `istype_checks_integer_type` | `tests/interpreter.rs` |
| `istype_any_faults_like_neovm` | `tests/interpreter.rs` |
| `isnull_detects_null` | `tests/interpreter.rs` |
| `mul_two_integers` | `tests/interpreter.rs` |
| `sqrt_accepts_big_integer_result_from_mul` | `tests/interpreter.rs` |
| `div_integers` | `tests/interpreter.rs` |
| `mod_integers` | `tests/interpreter.rs` |
| `abs_negative_integer` | `tests/interpreter.rs` |
| `dec_integer` | `tests/interpreter.rs` |
| `min_of_two` | `tests/interpreter.rs` |
| `max_of_two` | `tests/interpreter.rs` |
| `within_range` | `tests/interpreter.rs` |
| `booland_true_true` | `tests/interpreter.rs` |
| `boolor_false_true` | `tests/interpreter.rs` |
| `nz_nonzero` | `tests/interpreter.rs` |
| `nz_accepts_uint160_sized_bytestring` | `tests/interpreter.rs` |
| `substr_extracts_middle` | `tests/interpreter.rs` |
| `substr_preserves_buffer_type` | `tests/interpreter.rs` |
| `empty_buffer_substr_can_be_reversed_in_place` | `tests/interpreter.rs` |
| `right_extracts_suffix` | `tests/interpreter.rs` |
| `rot_rotates_top_three` | `tests/interpreter.rs` |
| `pick_duplicates_nth` | `tests/interpreter.rs` |
| `over_copies_second` | `tests/interpreter.rs` |
| `nip_removes_second` | `tests/interpreter.rs` |
| `tuck_copies_top_below_second` | `tests/interpreter.rs` |
| `clear_empties_stack` | `tests/interpreter.rs` |
| `reverse3_reverses_top_three` | `tests/interpreter.rs` |
| `reverse4_reverses_top_four` | `tests/interpreter.rs` |
| `throw_causes_fault` | `tests/interpreter.rs` |
| `abortmsg_includes_message` | `tests/interpreter.rs` |
| `ldarg_starg_round_trip` | `tests/interpreter.rs` |
| `pushint256_large_value` | `tests/interpreter.rs` |
| `convert_pushint256_to_buffer_preserves_big_integer_bytes` | `tests/interpreter.rs` |
| `gas_exhaustion_faults` | `tests/interpreter.rs` |
| `try_catch_catches_throw` | `tests/interpreter.rs` |
| `try_catch_catches_pickitem_type_error` | `tests/interpreter.rs` |
| `pickitem_reads_integer_as_little_endian_bytes` | `tests/interpreter.rs` |
| `pickitem_reads_negative_integer_sign_extended_bytes` | `tests/interpreter.rs` |
| `try_finally_executes_on_normal` | `tests/interpreter.rs` |
| `reserved_throwifnot_byte_is_rejected` | `tests/interpreter.rs` |
| `convert_integer_to_bytestring` | `tests/interpreter.rs` |
| `convert_wide_bytestring_to_integer_preserves_big_integer` | `tests/interpreter.rs` |
| `convert_buffer_larger_than_max_integer_size_faults` | `tests/interpreter.rs` |
| `popitem_removes_last_from_array` | `tests/interpreter.rs` |
| `memcpy_copies_bytes` | `tests/interpreter.rs` |
| `roll_moves_nth_to_top` | `tests/interpreter.rs` |
| `xdrop_removes_nth_from_top` | `tests/interpreter.rs` |
| `reversen_reverses_n_items` | `tests/interpreter.rs` |
| `pusha_pushes_address` | `tests/interpreter.rs` |
| `pushdata2_large_bytestring` | `tests/interpreter.rs` |
| `jmpgt_jumps_when_greater` | `tests/interpreter.rs` |
| `jmpge_jumps_when_equal` | `tests/interpreter.rs` |
| `jmplt_jumps_when_less` | `tests/interpreter.rs` |
| `jmple_jumps_when_equal` | `tests/interpreter.rs` |
| `jmp_l_long_jump` | `tests/interpreter.rs` |
| `call_l_long_call` | `tests/interpreter.rs` |
| `calla_calls_address` | `tests/interpreter.rs` |
| `legacy_reserved_stack_opcode_bytes_are_rejected` | `tests/interpreter.rs` |
| `assertmsg_true_passes` | `tests/interpreter.rs` |
| `assertmsg_false_faults_with_message` | `tests/interpreter.rs` |
| `try_l_long_form_catch` | `tests/interpreter.rs` |
| `endtry_l_long_form` | `tests/interpreter.rs` |
| `pushdata4_with_valid_payload` | `tests/interpreter.rs` |
| `callt_invokes_host` | `tests/interpreter.rs` |
| `callt_array_result_round_trips_through_locals_and_pickitem` | `tests/interpreter.rs` |
| `callt_array_result_survives_prior_syscall_with_live_args_in_interpreter` | `tests/interpreter.rs` |
| `syscall_invokes_host_provider` | `tests/interpreter.rs` |
| `slot_ldloc6_stloc6` | `tests/interpreter.rs` |
| `slot_ldsfld6_stsfld6` | `tests/interpreter.rs` |
| `slot_ldarg6_starg6` | `tests/interpreter.rs` |
| `jmpif_l_long_form` | `tests/interpreter.rs` |
| `jmpifnot_l_long_form` | `tests/interpreter.rs` |
| `push_high_constants` | `tests/interpreter.rs` |
| `xdrop_removes_nth_from_top_deep` | `tests/interpreter.rs` |
| `call_ret_preserves_caller_locals` | `tests/interpreter.rs` |
| `caller_catch_restores_locals_after_callee_throw` | `tests/interpreter.rs` |
| `callt_null_result_can_flow_through_two_arg_helper` | `tests/interpreter.rs` |
| `callt_block_like_struct_survives_local_and_helper_pickitem_in_interpreter` | `tests/interpreter.rs` |
| `try_finally_endfinally_continues_at_correct_ip` | `tests/interpreter.rs` |
| `jmpeq_jumps_on_deep_equal_structs` | `tests/interpreter.rs` |
| `test_rc_basic` | `tests/rc_compat_test.rs` |
| `test_rc_nested` | `tests/rc_compat_test.rs` |
| `test_rc_drop` | `tests/rc_compat_test.rs` |
| `guest_stack_value_is_the_shared_neo_vm_rs_type` | `tests/shared_vm_dependency.rs` |
| `guest_interpreter_is_exposed_from_neo_vm_rs` | `tests/shared_vm_dependency.rs` |
| `guest_crate_is_a_facade_not_a_private_interpreter` | `tests/shared_vm_dependency.rs` |
| `contract_runtime_is_part_of_guest_crate_not_a_separate_workspace_crate` | `tests/workspace_layering.rs` |

## Dependency Boundary

| Dependency | Kind |
| --- | --- |
| `neo-riscv-abi` | runtime |
| `neo-vm-rs` | runtime |

## Suggested Reading Path

1. Read `src/lib.rs`: crate root, public exports, and top-level documentation.
2. Read `tests/interpreter.rs`: external behavior or integration test.
3. Read `tests/contracts.rs`: external behavior or integration test.
4. Read `src/contract_rt/mod.rs`: implementation detail or helper module.
5. Read `tests/contract_rt_context.rs`: external behavior or integration test.
6. Read `tests/contract_rt_layering.rs`: external behavior or integration test.

## Change Safety Checklist

- Keep the stated responsibility boundary intact: Call shared VM runtime, Expose no_std contract APIs, Bridge syscalls.
- Update the workflow and dataflow diagrams when adding or removing major execution steps.
- Add or update tests in the files listed under Test Evidence when public API or state-transition behavior changes.
- Re-run `python tools/docs/generate_crate_visual_docs.py` from the Neo N4 repository root after source layout changes.
