# neo-riscv-fuzz

<!-- N4-CRATE-VISUAL-GUIDE-ZH:START -->

## 可视化学习指南

这些图是 `neo-riscv-fuzz` 自己目录下的 crate 专属学习资料，用来说明它在 Neo N4 中的位置、自己负责的技术边界、内部工作流，以及数据如何流经它。

| 视图 | 图片 | 源文件 |
| --- | --- | --- |
| 在 Neo N4 中的位置 | ![位置](docs/figures/position.zh.svg) | [Mermaid](docs/figures/position.zh.mmd) |
| 技术原理 | ![技术原理](docs/figures/principles.zh.svg) | [Mermaid](docs/figures/principles.zh.mmd) |
| 架构 | ![架构](docs/figures/architecture.zh.svg) | [Mermaid](docs/figures/architecture.zh.mmd) |
| 工作流 | ![工作流](docs/figures/workflow.zh.svg) | [Mermaid](docs/figures/workflow.zh.mmd) |
| 数据流 | ![数据流](docs/figures/dataflow.zh.svg) | [Mermaid](docs/figures/dataflow.zh.mmd) |

### 在 Neo N4 中的作用

- **层级:** NeoVM2 / RISC-V 执行 profile
- **目的:** 面向 RISC-V VM 执行、ABI codec、host/guest 边界的 fuzz 支撑。
- **主要输入:** 种子语料、生成 opcode、变异栈值
- **主要输出:** 回归 seed、崩溃案例、覆盖率信号
- **下游使用者:** RISC-V host、Neo N4 L2 节点、开发者工具

### 边界与职责

- **本 crate 负责:** 生成有效脚本、覆盖 codec、发现 host/guest 不一致
- **本 crate 消费:** 种子语料、生成 opcode、变异栈值
- **本 crate 产出:** 回归 seed、崩溃案例、覆盖率信号
- **主要被谁使用:** RISC-V host、Neo N4 L2 节点、开发者工具

### 学习路径

1. 先看位置图，明确这个 crate 为什么存在、上游是谁、下游是谁。
2. 再看技术原理图，理解它的核心不变量、职责边界和维护规则。
3. 然后看架构图，把公开入口、内部组件、依赖边界和输出产物串起来。
4. 最后看工作流和数据流，再进入源码和测试文件会更容易理解。

<!-- N4-CRATE-VISUAL-GUIDE-ZH:END -->
