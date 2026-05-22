# neo-riscv-host 技术学习指南

这份指南把 `neo-riscv-host` 当作 Neo N4 的一个技术单元来解释。它不是源码阅读图，而是帮助读者理解：这个单元负责什么、哪些技术假设保证它正确、数据如何移动、状态如何变化、证据如何被验证、它如何接入 Neo N4 的整体架构。

## 技术契约

| 维度 | 含义 |
| --- | --- |
| 层级 | NeoVM2 / RISC-V 执行 profile |
| 目的 | 执行 PolkaVM guest 模块、计费 gas 并桥接 syscall 的 host runtime。 |
| 输入 | PolkaVM 模块 <br> 执行上下文 <br> 宿主 syscall provider |
| 职责 | 实例化模块 <br> 按 opcode 计费 <br> 编组栈值 <br> 返回 VM 结果 |
| 输出 | 执行结果 <br> gas 报告 <br> host 轨迹 |
| 消费方 | RISC-V host <br> Neo N4 L2 节点 <br> 开发者工具 |

## 图表集合

| # | 图 | 学什么 |
| --- | --- | --- |
| 1 | [系统位置图](figures/position.zh.svg) | 它在 Neo N4 中的位置。 |
| 2 | [技术原理图](figures/principles.zh.svg) | 保证设计正确的技术规则。 |
| 3 | [概念架构图](figures/architecture.zh.svg) | 主要技术块和边界。 |
| 4 | [工作流图](figures/workflow.zh.svg) | 运行时的有序过程。 |
| 5 | [数据流图](figures/dataflow.zh.svg) | 信息、承诺和证据如何移动。 |
| 6 | [状态模型图](figures/state-model.zh.svg) | 状态归属、转换和终局性。 |
| 7 | [证明与证据流图](figures/proof-flow.zh.svg) | 声明如何变成可验证证据。 |
| 8 | [信任边界图](figures/trust-boundaries.zh.svg) | 哪些内容被信任、检查、拒绝或观测。 |
| 9 | [集成关系图](figures/integration-map.zh.svg) | 该单元如何接入更大的 N4 栈。 |
| 10 | [运行生命周期图](figures/lifecycle.zh.svg) | 从配置到执行、证据和运维的生命周期。 |

## 架构模型

`neo-riscv-host` 接收 PolkaVM 模块 | 执行上下文 | 宿主 syscall provider，拥有的边界是：实例化模块 | 按 opcode 计费 | 编组栈值 | 返回 VM 结果。它输出 执行结果 | gas 报告 | host 轨迹，然后由 RISC-V host | Neo N4 L2 节点 | 开发者工具 消费。

分层规则：guest 负责合约语义；host 负责资源、syscall 和链上下文。

## 工作流

1. 准备合约/输入
2. 编码 ABI
3. 在 PolkaVM 路径执行
4. 收集结果
5. 验证证据

失败路径：ABI 解码失败、host callback 拒绝、gas 耗尽或 guest fault。

## 数据流

1. 合约输入
2. neo-riscv-host
3. host/guest 边界
4. Neo N4 状态转换

承诺信号：ABI 摘要、执行结果、gas 报告和 syscall 轨迹。

## 状态、证明和信任

- 状态转换：状态通过 ABI、PolkaVM 执行和 host syscall 边界变化。
- 终局条件：host 接受结果并把状态变化纳入 L2 转换。
- 信任模型：信任 VM 与 host 规则，不信任 guest 输入或外部回调。
- 验证边界：ABI、gas、syscall、host context 和执行结果必须一致。
- 重放与顺序：执行上下文绑定调用、状态和 gas，避免跨上下文复用。

## 集成和运行

- NeoFS DA：NeoFS 保存批次数据、见证或轨迹摘要以及可取回证据。
- 证明系统：证明系统把 L2 执行声明压缩为可验证证据。
- Gateway/API：Gateway 负责用户路由、查询、提交和健康状态聚合。
- 桥与异构链：桥规则统一 L1-L2、L2-L2 和异构链消息与资产。
- 可观测证据：gas、syscall、host trace、halt/fault 和执行摘要。

在 Neo N4 仓库根目录重新生成这些技术图：

```powershell
python tools/docs/generate_crate_visual_docs.py
```
