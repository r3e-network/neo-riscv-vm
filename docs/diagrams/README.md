# Neo RISC-V VM Diagrams / Neo RISC-V VM 图解

Professional bilingual diagrams for explaining how NeoVM compatibility runs inside Neo's RISC-V execution stack.

用于解释 NeoVM 兼容层如何运行在 Neo RISC-V 执行栈中的专业中英文图解。

Regenerate all diagrams:

```bash
node docs/diagrams/generate-diagrams.mjs
```

## English Set

Generated SVG diagrams. Edit generate-diagrams.mjs, then rerun node docs/diagrams/generate-diagrams.mjs.

| Diagram | Image |
| --- | --- |
| System Architecture | ![System Architecture](./en/01-system-architecture.svg) |
| Execution Routing | ![Execution Routing](./en/02-execution-routing.svg) |
| NeoVM Inside RISC-V | ![NeoVM Inside RISC-V](./en/03-neovm-inside-riscv.svg) |
| Syscall and Native Contract Data Flow | ![Syscall and Native Contract Data Flow](./en/04-syscall-native-data-flow.svg) |
| ABI and Memory Model | ![ABI and Memory Model](./en/05-abi-memory-model.svg) |
| Contract Deployment Workflow | ![Contract Deployment Workflow](./en/06-contract-deployment-workflow.svg) |
| Mainnet State Root Validation | ![Mainnet State Root Validation](./en/07-mainnet-stateroot-validation.svg) |

## 中文图集

生成式 SVG 图集。修改 generate-diagrams.mjs 后运行 node docs/diagrams/generate-diagrams.mjs 重新生成。

| 图 | 图片 |
| --- | --- |
| 系统总体架构 | ![系统总体架构](./zh/01-system-architecture.svg) |
| 执行路由 | ![执行路由](./zh/02-execution-routing.svg) |
| NeoVM 运行在 RISC-V 中 | ![NeoVM 运行在 RISC-V 中](./zh/03-neovm-inside-riscv.svg) |
| Syscall 与原生合约数据流 | ![Syscall 与原生合约数据流](./zh/04-syscall-native-data-flow.svg) |
| ABI 与内存模型 | ![ABI 与内存模型](./zh/05-abi-memory-model.svg) |
| 合约部署工作流 | ![合约部署工作流](./zh/06-contract-deployment-workflow.svg) |
| 主网 StateRoot 校验 | ![主网 StateRoot 校验](./zh/07-mainnet-stateroot-validation.svg) |
