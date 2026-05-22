# neo-riscv-guest-module 源码级学习指南

这份文档从 crate 的真实 `Cargo.toml`、Rust 源码文件、公开符号和测试函数生成。目标是在读实现细节之前，先弄清楚这个 crate 自己负责什么、边界在哪里、应该从哪些文件开始读。

## 这个 Crate 是什么

| 主题 | 说明 |
| --- | --- |
| 层级 | NeoVM2 / RISC-V 执行 profile |
| 目的 | PolkaVM guest 模块入口，将 guest runtime 打包成可执行 RISC-V 代码。 |
| 输入 | PolkaVM import、guest runtime、编码执行输入 |
| 职责 | 暴露 export、调用 guest runtime、返回编码输出 |
| 输出 | guest.polkavm、编码输出、host callback 调用 |
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
| `src/main.rs` | 二进制或 CLI 入口 | 26 | 3 |
| `src/mem_intrinsics.rs` | 实现细节或辅助模块 | 0 | 0 |

## 公开 API 面

| 符号 | 文件 |
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

## 模块与重导出信号

| 信号 |
| --- |
| `src/main.rs: mod mem_intrinsics` |

## 测试证据

| 测试 | 文件 |
| --- | --- |
| `mainnet_470449_requires_heap_headroom_above_16_mib` | `src/main.rs` |
| `mainnet_2368696_requires_heap_headroom_above_32_mib` | `src/main.rs` |
| `mainnet_2655903_requires_heap_headroom_above_64_mib` | `src/main.rs` |

## 依赖边界

| 依赖 | 类型 |
| --- | --- |
| `neo-riscv-abi` | 运行时 |
| `neo-riscv-guest` | 运行时 |
| `polkavm-derive` | 运行时 |
| `postcard` | 运行时 |

## 建议阅读路径

1. 读 `src/main.rs`：二进制或 CLI 入口。
2. 读 `src/mem_intrinsics.rs`：实现细节或辅助模块。

## 修改安全清单

- 保持职责边界不变：暴露 export、调用 guest runtime、返回编码输出。
- 增加或删除主要执行步骤时，同步更新工作流图和数据流图。
- 修改公开 API 或状态转换行为时，更新“测试证据”中对应的测试。
- 源码结构变化后，在 Neo N4 仓库根目录重新运行 `python tools/docs/generate_crate_visual_docs.py`。
