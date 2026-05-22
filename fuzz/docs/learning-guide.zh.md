# neo-riscv-fuzz 源码级学习指南

这份文档从 crate 的真实 `Cargo.toml`、Rust 源码文件、公开符号和测试函数生成。目标是在读实现细节之前，先弄清楚这个 crate 自己负责什么、边界在哪里、应该从哪些文件开始读。

## 这个 Crate 是什么

| 主题 | 说明 |
| --- | --- |
| 层级 | NeoVM2 / RISC-V 执行 profile |
| 目的 | 面向 RISC-V VM 执行、ABI codec、host/guest 边界的 fuzz 支撑。 |
| 输入 | 种子语料、生成 opcode、变异栈值 |
| 职责 | 生成有效脚本、覆盖 codec、发现 host/guest 不一致 |
| 输出 | 回归 seed、崩溃案例、覆盖率信号 |
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
| `src/lib.rs` | crate 根、公开导出和顶层文档 | 8 | 1 |
| `src/generators/value_gen.rs` | 实现细节或辅助模块 | 4 | 4 |
| `src/generators/opcode_gen.rs` | opcode 元数据、定价或标准解码规则 | 3 | 4 |
| `src/stack_ops_builder.rs` | 实现细节或辅助模块 | 1 | 2 |
| `src/whole_system_parity.rs` | 实现细节或辅助模块 | 0 | 4 |
| `src/syscall_fuzz.rs` | fuzz harness 与对抗输入探索 | 0 | 1 |
| `src/exception_handling.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/generators.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/mem_op.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/opcode_seq.rs` | opcode 元数据、定价或标准解码规则 | 0 | 0 |
| `src/stack_ops.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/type_convert.rs` | 实现细节或辅助模块 | 0 | 0 |

## 公开 API 面

| 符号 | 文件 |
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

## 模块与重导出信号

| 信号 |
| --- |
| `src/generators.rs: mod opcode_gen` |
| `src/generators.rs: mod value_gen` |
| `src/generators.rs: pub use opcode_gen::*` |
| `src/generators.rs: pub use value_gen::*` |
| `src/lib.rs: mod generators` |
| `src/lib.rs: mod stack_ops_builder` |
| `src/stack_ops.rs: mod stack_ops_builder` |

## 测试证据

| 测试 | 文件 |
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

## 依赖边界

| 依赖 | 类型 |
| --- | --- |
| `libfuzzer-sys` | 运行时 |
| `neo-riscv-abi` | 运行时 |
| `neo-riscv-guest` | 运行时 |
| `neo-riscv-host` | 运行时 |
| `serde` | 运行时 |
| `serde_json` | 运行时 |

## 建议阅读路径

1. 读 `src/lib.rs`：crate 根、公开导出和顶层文档。
2. 读 `src/generators/value_gen.rs`：实现细节或辅助模块。
3. 读 `src/generators/opcode_gen.rs`：opcode 元数据、定价或标准解码规则。
4. 读 `src/stack_ops_builder.rs`：实现细节或辅助模块。
5. 读 `src/whole_system_parity.rs`：实现细节或辅助模块。
6. 读 `src/syscall_fuzz.rs`：fuzz harness 与对抗输入探索。

## 修改安全清单

- 保持职责边界不变：生成有效脚本、覆盖 codec、发现 host/guest 不一致。
- 增加或删除主要执行步骤时，同步更新工作流图和数据流图。
- 修改公开 API 或状态转换行为时，更新“测试证据”中对应的测试。
- 源码结构变化后，在 Neo N4 仓库根目录重新运行 `python tools/docs/generate_crate_visual_docs.py`。
