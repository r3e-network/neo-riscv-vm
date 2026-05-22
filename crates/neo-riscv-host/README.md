# neo-riscv-host

<!-- N4-CRATE-VISUAL-GUIDE:START -->

## Crate Visual Learning Guide

These diagrams are local to this crate. They explain `neo-riscv-host` as an independent unit: where it sits in the Neo N4 stack, which boundary it owns, how its internal workflow runs, and how data moves through it.

| View | Diagram | Source |
| --- | --- | --- |
| Position in Neo N4 | ![Position](docs/figures/position.svg) | [Mermaid](docs/figures/position.mmd) |
| Technical principles | ![Principles](docs/figures/principles.svg) | [Mermaid](docs/figures/principles.mmd) |
| Architecture | ![Architecture](docs/figures/architecture.svg) | [Mermaid](docs/figures/architecture.mmd) |
| Workflow | ![Workflow](docs/figures/workflow.svg) | [Mermaid](docs/figures/workflow.mmd) |
| Dataflow | ![Dataflow](docs/figures/dataflow.svg) | [Mermaid](docs/figures/dataflow.mmd) |

### Role in Neo N4

- **Layer:** NeoVM2 / RISC-V execution profile
- **Purpose:** Host runtime that executes PolkaVM guest modules, accounts gas, and bridges syscalls.
- **Primary inputs:** PolkaVM module, execution context, host syscall provider
- **Primary outputs:** execution result, gas report, host trace
- **Downstream consumers:** RISC-V host, Neo N4 L2 node, developer tooling

### Boundary and Responsibilities

- **Owns:** Instantiate module, Charge opcodes, Marshal stack values, Return VM result
- **Consumes:** PolkaVM module, execution context, host syscall provider
- **Produces:** execution result, gas report, host trace
- **Used by:** RISC-V host, Neo N4 L2 node, developer tooling

### Learning Path

1. Start with the position diagram to understand why this crate exists and who calls it.
2. Read the technical principles diagram to identify the invariants and responsibility boundary.
3. Use the architecture diagram to connect public inputs, internal components, dependencies, and outputs.
4. Follow the workflow and dataflow diagrams before reading source files or tests.

<!-- N4-CRATE-VISUAL-GUIDE:END -->
