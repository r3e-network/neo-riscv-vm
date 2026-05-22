# counter

<!-- N4-CRATE-VISUAL-GUIDE-ZH:START -->
## 技术可视化指南

这些图都放在本 crate 目录下，用技术架构视角解释 `counter`。重点是系统位置、技术原理、数据移动、工作流、状态、证明/证据、信任边界、集成关系和运行生命周期。

完整技术解释见 [docs/learning-guide.zh.md](docs/learning-guide.zh.md)。

| 视图 | 图 | Mermaid |
| --- | --- | --- |
| 系统位置图 | ![系统位置图](docs/figures/position.zh.svg) | [Mermaid](docs/figures/position.zh.mmd) |
| 技术原理图 | ![技术原理图](docs/figures/principles.zh.svg) | [Mermaid](docs/figures/principles.zh.mmd) |
| 概念架构图 | ![概念架构图](docs/figures/architecture.zh.svg) | [Mermaid](docs/figures/architecture.zh.mmd) |
| 工作流图 | ![工作流图](docs/figures/workflow.zh.svg) | [Mermaid](docs/figures/workflow.zh.mmd) |
| 数据流图 | ![数据流图](docs/figures/dataflow.zh.svg) | [Mermaid](docs/figures/dataflow.zh.mmd) |
| 状态模型图 | ![状态模型图](docs/figures/state-model.zh.svg) | [Mermaid](docs/figures/state-model.zh.mmd) |
| 证明与证据流图 | ![证明与证据流图](docs/figures/proof-flow.zh.svg) | [Mermaid](docs/figures/proof-flow.zh.mmd) |
| 信任边界图 | ![信任边界图](docs/figures/trust-boundaries.zh.svg) | [Mermaid](docs/figures/trust-boundaries.zh.mmd) |
| 集成关系图 | ![集成关系图](docs/figures/integration-map.zh.svg) | [Mermaid](docs/figures/integration-map.zh.mmd) |
| 运行生命周期图 | ![运行生命周期图](docs/figures/lifecycle.zh.svg) | [Mermaid](docs/figures/lifecycle.zh.mmd) |

### 技术角色

- **层级:** NeoVM2 / RISC-V 执行 profile
- **目的:** 最小状态变更 counter 合约示例。
- **输入:** 开发者源码 | 模板 runtime | 测试上下文
- **职责:** 演示合约模式 | 覆盖工具链 | 提供学习 fixture
- **输出:** 编译后合约 | 示例结果 | 教程产物
- **消费方:** RISC-V host | Neo N4 L2 节点 | 开发者工具

### 阅读顺序

1. 先看系统位置图和概念架构图。
2. 再看技术原理图、信任边界图和状态模型图，理解为什么这样设计是正确的。
3. 然后看工作流图和数据流图，理解运行时如何移动。
4. 最后看证明/证据流、集成关系和生命周期，理解系统如何进入真实运行。
<!-- N4-CRATE-VISUAL-GUIDE-ZH:END -->
