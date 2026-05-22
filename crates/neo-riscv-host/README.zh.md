# neo-riscv-host

<!-- N4-CRATE-VISUAL-GUIDE-ZH:START -->

## 可视化架构学习指南

这些图用于说明 `neo-riscv-host` 在 Neo N4 栈中的位置、主要工作流，以及数据如何流经该 crate。

| 视图 | 图片 | 源文件 |
| --- | --- | --- |
| 架构 | ![架构](docs/figures/architecture.zh.svg) | [Mermaid](docs/figures/architecture.zh.mmd) |
| 工作流 | ![工作流](docs/figures/workflow.zh.svg) | [Mermaid](docs/figures/workflow.zh.mmd) |
| 数据流 | ![数据流](docs/figures/dataflow.zh.svg) | [Mermaid](docs/figures/dataflow.zh.mmd) |

### 在 Neo N4 中的作用

- **层级:** NeoVM2 / RISC-V 执行 profile
- **目的:** 执行 PolkaVM guest 模块、计费 gas 并桥接 syscall 的 host runtime。
- **主要输入:** PolkaVM 模块、执行上下文、宿主 syscall provider
- **主要输出:** 执行结果、gas 报告、host 轨迹
- **下游消费者:** RISC-V host、Neo N4 L2 节点、开发者工具

### 学习路径

1. 先看架构图，理解 crate 的边界和所在层级。
2. 再看工作流图，理解正常执行路径。
3. 最后看数据流图，把输入、状态变化和输出串起来。
4. 带着图中的上下文阅读源码，会更容易理解模块职责。

<!-- N4-CRATE-VISUAL-GUIDE-ZH:END -->
