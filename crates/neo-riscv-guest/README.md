# neo-riscv-guest

<!-- N4-CRATE-VISUAL-GUIDE:START -->

## Crate Visual Learning Guide

These diagrams are local to this crate. They explain `neo-riscv-guest` as an independent unit: where it sits in the Neo N4 stack, which boundary it owns, how its internal workflow runs, and how data moves through it.

| View | Diagram | Source |
| --- | --- | --- |
| Position in Neo N4 | ![Position](docs/figures/position.svg) | [Mermaid](docs/figures/position.mmd) |
| Technical principles | ![Principles](docs/figures/principles.svg) | [Mermaid](docs/figures/principles.mmd) |
| Architecture | ![Architecture](docs/figures/architecture.svg) | [Mermaid](docs/figures/architecture.mmd) |
| Workflow | ![Workflow](docs/figures/workflow.svg) | [Mermaid](docs/figures/workflow.mmd) |
| Dataflow | ![Dataflow](docs/figures/dataflow.svg) | [Mermaid](docs/figures/dataflow.mmd) |

### Role in Neo N4

- **Layer:** NeoVM2 / RISC-V execution profile
- **Purpose:** Guest-side facade and contract runtime glue for NeoVM2/RISC-V contracts.
- **Primary inputs:** contract bytecode, ABI stack, syscall stubs
- **Primary outputs:** guest result, syscall request, stack mutation
- **Downstream consumers:** RISC-V host, Neo N4 L2 node, developer tooling

### Boundary and Responsibilities

- **Owns:** Call shared VM runtime, Expose no_std contract APIs, Bridge syscalls
- **Consumes:** contract bytecode, ABI stack, syscall stubs
- **Produces:** guest result, syscall request, stack mutation
- **Used by:** RISC-V host, Neo N4 L2 node, developer tooling

### Learning Path

1. Start with the position diagram to understand why this crate exists and who calls it.
2. Read the technical principles diagram to identify the invariants and responsibility boundary.
3. Use the architecture diagram to connect public inputs, internal components, dependencies, and outputs.
4. Follow the workflow and dataflow diagrams before reading source files or tests.

<!-- N4-CRATE-VISUAL-GUIDE:END -->
