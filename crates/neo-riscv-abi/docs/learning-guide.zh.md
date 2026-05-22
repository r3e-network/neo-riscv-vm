# neo-riscv-abi 源码级学习指南

这份文档从 crate 的真实 `Cargo.toml`、Rust 源码文件、公开符号和测试函数生成。目标是在读实现细节之前，先弄清楚这个 crate 自己负责什么、边界在哪里、应该从哪些文件开始读。

## 这个 Crate 是什么

| 主题 | 说明 |
| --- | --- |
| 层级 | NeoVM2 / RISC-V 执行 profile |
| 目的 | RISC-V 执行路径共享的 ABI、栈值、codec tag 与 opcode 元数据重导出。 |
| 输入 | 共享 VM 类型、host/guest 边界、序列化栈值 |
| 职责 | 定义稳定 ABI、重导出共享元数据、编码/解码栈值 |
| 输出 | ABI 类型、codec helper、运行时常量 |
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
| 10 | [实现全景图](figures/implementation-atlas.zh.svg) | 用一张高密度图同时理解用途、源码入口、API、工作流、数据流、依赖、测试和修改检查点。 |

## 源码文件地图

| 文件 | 作用 | 公开符号 | 测试 |
| --- | --- | ---: | ---: |
| `src/lib.rs` | crate 根、公开导出和顶层文档 | 0 | 2 |
| `tests/codec_tests.rs` | 外部行为或集成测试 | 0 | 41 |
| `tests/shared_vm_codecs.rs` | 外部行为或集成测试 | 0 | 2 |
| `src/callback_codec.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/fast_codec.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/result_codec.rs` | 实现细节或辅助模块 | 0 | 0 |

## 公开 API 面

未扫描到公开 Rust 符号。

## 模块与重导出信号

| 信号 |
| --- |
| `src/callback_codec.rs: pub use neo_vm_rs::callback_codec::*` |
| `src/fast_codec.rs: pub use neo_vm_rs::fast_codec::*` |
| `src/lib.rs: mod callback_codec` |
| `src/lib.rs: mod fast_codec` |
| `src/lib.rs: mod result_codec` |
| `src/lib.rs: pub use neo_vm_rs::semantics` |
| `src/lib.rs: pub use neo_vm_rs::{     byte_sequence_bytes, byte_sequence_len, concat_byte_sequences, default_value_for_type_tag,     encode_integer, interop_hash, new_array_default_value_for_type_tag,     normalize_stack_item_type_tag, slice_byte_sequence, stack_value_as_bool, stack_value_as_bytes,     stack_value_as_fixed_bytes, stack_value_as_i64, stack_value_as_string, stack_value_as_u32,     stack_value_as_u8, stack_value_into_items, syscall_arg_count, BackendKind, ExecutionResult,     OpCode, StackValue, VmContext, VmState, COMPACT_TAG_ARRAY as TAG_ARRAY,     COMPACT_TAG_BIG_INTEGER as TAG_BIG_INTEGER, COMPACT_TAG_BOOLEAN as TAG_BOOLEAN,     COMPACT_TAG_BUFFER as TAG_BUFFER, COMPACT_TAG_BYTESTRING as TAG_BYTESTRING,     COMPACT_TAG_INTEGER as TAG_INTEGER, COMPACT_TAG_INTEROP as TAG_INTEROP,     COMPACT_TAG_ITERATOR as TAG_ITERATOR, COMPACT_TAG_MAP as TAG_MAP, COMPACT_TAG_NULL as TAG_NULL,     COMPACT_TAG_POINTER as TAG_POINTER, COMPACT_TAG_STRUCT as TAG_STRUCT,     STACK_VALUE_CODEC_TAG_ARRAY, STACK_VALUE_CODEC_TAG_BIG_INTEGER, STACK_VALUE_CODEC_TAG_BOOLEAN,     STACK_VALUE_CODEC_TAG_BUFFER, STACK_VALUE_CODEC_TAG_BYTESTRING, STACK_VALUE_CODEC_TAG_INTEGER,     STACK_VALUE_CODEC_TAG_INTEROP, STACK_VALUE_CODEC_TAG_ITERATOR, STACK_VALUE_CODEC_TAG_MAP,     STACK_VALUE_CODEC_TAG_NULL, STACK_VALUE_CODEC_TAG_POINTER, STACK_VALUE_CODEC_TAG_STRUCT, }` |
| `src/result_codec.rs: pub use neo_vm_rs::result_codec::*` |

## 测试证据

| 测试 | 文件 |
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

## 依赖边界

| 依赖 | 类型 |
| --- | --- |
| `neo-vm-rs` | 运行时 |
| `sha2` | 测试 |

## 建议阅读路径

1. 读 `src/lib.rs`：crate 根、公开导出和顶层文档。
2. 读 `tests/codec_tests.rs`：外部行为或集成测试。
3. 读 `tests/shared_vm_codecs.rs`：外部行为或集成测试。
4. 读 `src/callback_codec.rs`：实现细节或辅助模块。
5. 读 `src/fast_codec.rs`：实现细节或辅助模块。
6. 读 `src/result_codec.rs`：实现细节或辅助模块。

## 修改安全清单

- 保持职责边界不变：定义稳定 ABI、重导出共享元数据、编码/解码栈值。
- 增加或删除主要执行步骤时，同步更新工作流图和数据流图。
- 修改公开 API 或状态转换行为时，更新“测试证据”中对应的测试。
- 源码结构变化后，在 Neo N4 仓库根目录重新运行 `python tools/docs/generate_crate_visual_docs.py`。
