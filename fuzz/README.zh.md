# neo-riscv-fuzz

<!-- N4-CRATE-VISUAL-GUIDE-ZH:START -->

## 可视化学习指南

这些图是 `neo-riscv-fuzz` 自己目录下的 crate 专属学习资料，用来说明它在 Neo N4 中的位置、自己负责的技术边界、内部工作流，以及数据如何流经它。

完整的源码级解释见 [docs/learning-guide.zh.md](docs/learning-guide.zh.md)。

| 视图 | 图片 | 源文件 |
| --- | --- | --- |
| 在 Neo N4 中的位置 | ![位置](docs/figures/position.zh.svg) | [Mermaid](docs/figures/position.zh.mmd) |
| 技术原理 | ![技术原理](docs/figures/principles.zh.svg) | [Mermaid](docs/figures/principles.zh.mmd) |
| 架构 | ![架构](docs/figures/architecture.zh.svg) | [Mermaid](docs/figures/architecture.zh.mmd) |
| 工作流 | ![工作流](docs/figures/workflow.zh.svg) | [Mermaid](docs/figures/workflow.zh.mmd) |
| 数据流 | ![数据流](docs/figures/dataflow.zh.svg) | [Mermaid](docs/figures/dataflow.zh.mmd) |
| 模块图 | ![模块图](docs/figures/module-map.zh.svg) | [Mermaid](docs/figures/module-map.zh.mmd) |
| 公开 API 图 | ![公开 API 图](docs/figures/api-surface.zh.svg) | [Mermaid](docs/figures/api-surface.zh.mmd) |
| 测试证据图 | ![测试证据图](docs/figures/test-map.zh.svg) | [Mermaid](docs/figures/test-map.zh.mmd) |
| 依赖图 | ![依赖图](docs/figures/dependency-map.zh.svg) | [Mermaid](docs/figures/dependency-map.zh.mmd) |

### 在 Neo N4 中的作用

- **层级:** NeoVM2 / RISC-V 执行 profile
- **目的:** 面向 RISC-V VM 执行、ABI codec、host/guest 边界的 fuzz 支撑。
- **主要输入:** 种子语料、生成 opcode、变异栈值
- **主要输出:** 回归 seed、崩溃案例、覆盖率信号
- **下游使用者:** RISC-V host、Neo N4 L2 节点、开发者工具
- **扫描到的源码文件:** 12
- **扫描到的公开符号:** 16
- **扫描到的 Rust 测试:** 16

### 边界与职责

- **本 crate 负责:** 生成有效脚本、覆盖 codec、发现 host/guest 不一致
- **本 crate 消费:** 种子语料、生成 opcode、变异栈值
- **本 crate 产出:** 回归 seed、崩溃案例、覆盖率信号
- **主要被谁使用:** RISC-V host、Neo N4 L2 节点、开发者工具

### 源码地图快照

| 文件 | 为什么重要 | 公开 API | 测试 |
| --- | --- | ---: | ---: |
| `src/lib.rs` | crate 根、公开导出和顶层文档 | 8 | 1 |
| `src/generators/value_gen.rs` | 实现细节或辅助模块 | 4 | 4 |
| `src/generators/opcode_gen.rs` | opcode 元数据、定价或标准解码规则 | 3 | 4 |
| `src/stack_ops_builder.rs` | 实现细节或辅助模块 | 1 | 2 |
| `src/whole_system_parity.rs` | 实现细节或辅助模块 | 0 | 4 |
| `src/syscall_fuzz.rs` | fuzz harness 与对抗输入探索 | 0 | 1 |
| `src/exception_handling.rs` | 实现细节或辅助模块 | 0 | 0 |
| `src/generators.rs` | 实现细节或辅助模块 | 0 | 0 |

### API 快照

| 类型 | 代表符号 |
| --- | --- |
| 类型 | NoOpSyscall <br> SimpleRng |
| 函数 | is_valid_opcode <br> requires_immediate <br> generate_valid_script <br> generate_integer +10 |
| Trait | 未扫描到公开符号 |
| 常量 | 未扫描到公开符号 |

### 学习路径

1. 先看位置图，明确这个 crate 为什么存在、上游是谁、下游是谁。
2. 再看技术原理图，理解它的核心不变量、职责边界和维护规则。
3. 然后看模块图和 API 图，确定先读哪些文件、哪些符号。
4. 最后看工作流、数据流、测试证据图和依赖图，再进入源码会更容易理解。

<!-- N4-CRATE-VISUAL-GUIDE-ZH:END -->
