# neo-riscv-guest Technical Learning Guide

This guide explains `neo-riscv-guest` as a Neo N4 technical unit. It is written for architecture learning: what the unit is responsible for, which assumptions make it correct, how data moves, how state changes, how evidence is checked, and where it plugs into the wider Neo N4 stack.

## Technical Contract

| Aspect | Meaning |
| --- | --- |
| Layer | NeoVM2 / RISC-V execution profile |
| Purpose | Guest-side facade and contract runtime glue for NeoVM2/RISC-V contracts. |
| Inputs | contract bytecode <br> ABI stack <br> syscall stubs |
| Responsibilities | Call shared VM runtime <br> Expose no_std contract APIs <br> Bridge syscalls |
| Outputs | guest result <br> syscall request <br> stack mutation |
| Consumers | RISC-V host <br> Neo N4 L2 node <br> developer tooling |

## Diagram Set

| # | Diagram | What to learn |
| --- | --- | --- |
| 1 | [System Position](figures/position.svg) | where this crate sits in Neo N4. |
| 2 | [Technical Principles](figures/principles.svg) | the rules that make the design correct. |
| 3 | [Conceptual Architecture](figures/architecture.svg) | major technical blocks and boundaries. |
| 4 | [Workflow](figures/workflow.svg) | the ordered runtime process. |
| 5 | [Data Flow](figures/dataflow.svg) | how information, commitments, and evidence move. |
| 6 | [State Model](figures/state-model.svg) | state ownership, transitions, and finality. |
| 7 | [Proof and Evidence Flow](figures/proof-flow.svg) | how claims become verifiable evidence. |
| 8 | [Trust Boundaries](figures/trust-boundaries.svg) | what is trusted, checked, rejected, or observed. |
| 9 | [Integration Map](figures/integration-map.svg) | how this unit connects to the wider N4 stack. |
| 10 | [Runtime Lifecycle](figures/lifecycle.svg) | from configuration through execution, evidence, and operation. |

## Architecture Model

`neo-riscv-guest` receives contract bytecode | ABI stack | syscall stubs and owns this boundary: Call shared VM runtime | Expose no_std contract APIs | Bridge syscalls. It emits guest result | syscall request | stack mutation, which are consumed by RISC-V host | Neo N4 L2 node | developer tooling.

Layering rule: guest owns contract semantics; host owns resources, syscalls, and chain context.

## Workflow

1. Prepare contract/input
2. Encode ABI
3. Execute in PolkaVM path
4. Collect result
5. Validate evidence

Failure path: ABI decode fails, host callback rejects, gas is exhausted, or guest faults.

## Data Flow

1. contract input
2. neo-riscv-guest
3. host/guest boundary
4. Neo N4 state transition

Commitment signal: ABI digest, execution result, gas report, and syscall trace.

## State, Proof, and Trust

- State transition: state changes through ABI, PolkaVM execution, and host syscall boundaries.
- Finality: host accepts result and includes state change in L2 transition.
- Trust model: trust VM and host rules, not guest inputs or external callbacks.
- Validation boundary: ABI, gas, syscall, host context, and execution result must match.
- Replay and ordering: execution context binds call, state, and gas to prevent cross-context reuse.

## Integration and Operation

- NeoFS DA: NeoFS stores batch data, witness or trace summaries, and retrievable evidence.
- Proof system: The proof system compresses L2 execution claims into verifiable evidence.
- Gateway/API: Gateway handles user routing, queries, submission, and health aggregation.
- Bridge and heterogeneous chains: Bridge rules unify L1-L2, L2-L2, and heterogeneous-chain messages and assets.
- Observable evidence: gas, syscalls, host trace, halt/fault, and execution digest.

Regenerate these technical diagrams from the Neo N4 repository root with:

```powershell
python tools/docs/generate_crate_visual_docs.py
```
