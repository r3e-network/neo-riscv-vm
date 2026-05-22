# neo-riscv-devpack 源码级学习指南

这份文档从 crate 的真实 `Cargo.toml`、Rust 源码文件、公开符号和测试函数生成。目标是在读实现细节之前，先弄清楚这个 crate 自己负责什么、边界在哪里、应该从哪些文件开始读。

## 这个 Crate 是什么

| 主题 | 说明 |
| --- | --- |
| 层级 | NeoVM2 / RISC-V 执行 profile |
| 目的 | 用于编译和准备 RISC-V Neo 合约的开发者打包工具。 |
| 输入 | 合约源码、模板配置、工具链设置 |
| 职责 | 构建产物、校验元数据、打包部署文件 |
| 输出 | 合约包、manifest、开发诊断 |
| 使用者 | RISC-V host、Neo N4 L2 节点、开发者工具 |

## 可视化阅读顺序

| 步骤 | 图 | 用它学习什么 |
| ---: | --- | --- |
| 1 | [位置图](figures/position.zh.svg) | 这个 crate 为什么存在、在 Neo N4 中处于哪里。 |
| 2 | [技术原理图](figures/principles.zh.svg) | 这个 crate 必须保护的不变量和职责边界。 |
| 3 | [模块图](figures/module-map.zh.svg) | 哪些源码文件是最好的入口。 |
| 4 | [公开 API 图](figures/api-surface.zh.svg) | 哪些导出符号构成 crate 契约。 |
| 5 | [架构图](figures/architecture.zh.svg) | 输入、内部组件、依赖和输出如何连接。 |
| 6 | [工作流图](figures/workflow.zh.svg) | 正常执行路径。 |
| 7 | [数据流图](figures/dataflow.zh.svg) | 数据如何跨越 crate 边界并被转换。 |
| 8 | [测试证据图](figures/test-map.zh.svg) | 哪些测试保护行为。 |
| 9 | [依赖图](figures/dependency-map.zh.svg) | 哪些依赖是运行时、测试或构建期依赖。 |

## 源码文件地图

| 文件 | 作用 | 公开符号 | 测试 |
| --- | --- | ---: | ---: |
| `src/lib.rs` | crate 根、公开导出和顶层文档 | 0 | 0 |
| `src/api_ids.rs` | 实现细节或辅助模块 | 43 | 0 |
| `src/syscalls.rs` | 宿主 syscall 契约与分发边界 | 34 | 0 |
| `src/native/std_lib.rs` | 实现细节或辅助模块 | 13 | 0 |
| `src/storage.rs` | 实现细节或辅助模块 | 11 | 0 |
| `src/native/neo_token.rs` | 实现细节或辅助模块 | 10 | 0 |
| `src/native/policy.rs` | 实现细节或辅助模块 | 9 | 0 |
| `src/native/notary.rs` | 实现细节或辅助模块 | 8 | 0 |
| `src/codec.rs` | 实现细节或辅助模块 | 7 | 0 |
| `src/native/crypto_lib.rs` | 实现细节或辅助模块 | 7 | 0 |
| `src/events.rs` | 实现细节或辅助模块 | 6 | 0 |
| `src/native/gas_token.rs` | 实现细节或辅助模块 | 6 | 0 |
| `src/native/ledger.rs` | 实现细节或辅助模块 | 6 | 0 |
| `src/native/mod.rs` | 实现细节或辅助模块 | 5 | 1 |
| `src/native/contract_management.rs` | 实现细节或辅助模块 | 5 | 0 |
| `src/native/treasury.rs` | 实现细节或辅助模块 | 4 | 0 |
| `src/parser.rs` | 实现细节或辅助模块 | 4 | 0 |
| `tests/native_wrappers_test.rs` | 外部行为或集成测试 | 1 | 9 |
| `tests/syscalls_test.rs` | 外部行为或集成测试 | 1 | 9 |
| `src/signing.rs` | 实现细节或辅助模块 | 3 | 0 |
| `src/types.rs` | 实现细节或辅助模块 | 3 | 0 |
| `src/native/oracle.rs` | 实现细节或辅助模块 | 2 | 0 |
| `src/native/role_management.rs` | 实现细节或辅助模块 | 2 | 0 |
| `tests/syscalls_shape_test.rs` | 外部行为或集成测试 | 1 | 2 |
| `src/ffi.rs` | 实现细节或辅助模块 | 1 | 0 |
| `tests/api_ids_test.rs` | 外部行为或集成测试 | 0 | 1 |

## 公开 API 面

| 符号 | 文件 |
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

## 模块与重导出信号

| 信号 |
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

## 测试证据

| 测试 | 文件 |
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

## 依赖边界

| 依赖 | 类型 |
| --- | --- |
| `neo-riscv-abi` | 运行时 |

## 建议阅读路径

1. 读 `src/lib.rs`：crate 根、公开导出和顶层文档。
2. 读 `src/api_ids.rs`：实现细节或辅助模块。
3. 读 `src/syscalls.rs`：宿主 syscall 契约与分发边界。
4. 读 `src/native/std_lib.rs`：实现细节或辅助模块。
5. 读 `src/storage.rs`：实现细节或辅助模块。
6. 读 `src/native/neo_token.rs`：实现细节或辅助模块。

## 修改安全清单

- 保持职责边界不变：构建产物、校验元数据、打包部署文件。
- 增加或删除主要执行步骤时，同步更新工作流图和数据流图。
- 修改公开 API 或状态转换行为时，更新“测试证据”中对应的测试。
- 源码结构变化后，在 Neo N4 仓库根目录重新运行 `python tools/docs/generate_crate_visual_docs.py`。
