# neo-riscv-abi

<!-- N4-CRATE-VISUAL-GUIDE:START -->

## Crate Visual Learning Guide

These diagrams are local to this crate. They explain `neo-riscv-abi` as an independent unit: where it sits in the Neo N4 stack, which boundary it owns, how its internal workflow runs, and how data moves through it.

For the full source-level explanation, read [docs/learning-guide.md](docs/learning-guide.md).

| View | Diagram | Source |
| --- | --- | --- |
| Position in Neo N4 | ![Position](docs/figures/position.svg) | [Mermaid](docs/figures/position.mmd) |
| Technical principles | ![Principles](docs/figures/principles.svg) | [Mermaid](docs/figures/principles.mmd) |
| Architecture | ![Architecture](docs/figures/architecture.svg) | [Mermaid](docs/figures/architecture.mmd) |
| Workflow | ![Workflow](docs/figures/workflow.svg) | [Mermaid](docs/figures/workflow.mmd) |
| Dataflow | ![Dataflow](docs/figures/dataflow.svg) | [Mermaid](docs/figures/dataflow.mmd) |
| Module map | ![Module map](docs/figures/module-map.svg) | [Mermaid](docs/figures/module-map.mmd) |
| Public API surface | ![Public API surface](docs/figures/api-surface.svg) | [Mermaid](docs/figures/api-surface.mmd) |
| Test evidence | ![Test evidence](docs/figures/test-map.svg) | [Mermaid](docs/figures/test-map.mmd) |
| Dependency map | ![Dependency map](docs/figures/dependency-map.svg) | [Mermaid](docs/figures/dependency-map.mmd) |

### Role in Neo N4

- **Layer:** NeoVM2 / RISC-V execution profile
- **Purpose:** Shared ABI, stack values, codec tags, and opcode metadata re-exports for RISC-V execution.
- **Primary inputs:** shared VM types, host/guest boundary, serialized stack values
- **Primary outputs:** ABI types, codec helpers, runtime constants
- **Downstream consumers:** RISC-V host, Neo N4 L2 node, developer tooling
- **Source files scanned:** 6
- **Public symbols scanned:** 0
- **Rust tests scanned:** 45

### Boundary and Responsibilities

- **Owns:** Define stable ABI, Re-export shared metadata, Encode/decode stack values
- **Consumes:** shared VM types, host/guest boundary, serialized stack values
- **Produces:** ABI types, codec helpers, runtime constants
- **Used by:** RISC-V host, Neo N4 L2 node, developer tooling

### Source Map Snapshot

| File | Why it matters | Public API | Tests |
| --- | --- | ---: | ---: |
| `src/lib.rs` | crate root, public exports, and top-level documentation | 0 | 2 |
| `tests/codec_tests.rs` | external behavior or integration test | 0 | 41 |
| `tests/shared_vm_codecs.rs` | external behavior or integration test | 0 | 2 |
| `src/callback_codec.rs` | implementation detail or helper module | 0 | 0 |
| `src/fast_codec.rs` | implementation detail or helper module | 0 | 0 |
| `src/result_codec.rs` | implementation detail or helper module | 0 | 0 |

### API Snapshot

| Kind | Representative symbols |
| --- | --- |
| Types | no public symbols scanned |
| Functions | no public symbols scanned |
| Trait | no public symbols scanned |
| Constants | no public symbols scanned |

### Learning Path

1. Start with the position diagram to understand why this crate exists and who calls it.
2. Read the technical principles diagram to identify the invariants and responsibility boundary.
3. Use the module map and API surface to identify the files and symbols to read first.
4. Follow the workflow, dataflow, test, and dependency diagrams before changing code.

<!-- N4-CRATE-VISUAL-GUIDE:END -->
