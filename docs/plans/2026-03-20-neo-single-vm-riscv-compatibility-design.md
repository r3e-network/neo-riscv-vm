# Neo Single-VM RISC-V Compatibility Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the PolkaVM-backed RISC-V runtime the only execution VM used by Neo N3 core, while preserving 100% backward compatibility for existing NeoVM contracts, NEF artifacts, manifests, RPC flows, wallets, witnesses, script builders, and contract-call tooling.

**Architecture:** Neo core always executes through one outer RISC-V VM. Existing NeoVM contracts are not executed directly by C# or a second core VM; instead, Neo routes them into a protocol-owned legacy-VM contract image that runs on the outer RISC-V VM and interprets legacy Neo bytecode. New RISC-V contracts are carried in the same NEF envelope, selected by optional metadata, and use the same chain state, syscall, storage, notification, and fee-policy surfaces as legacy contracts.

**Tech Stack:** `neo-project/neo` `master-n3`, Rust host runtime in `/home/neo/git/neo-riscv-vm`, PolkaVM interpreter backend, standard NEF + manifest artifacts, optional manifest extra metadata for new RISC-V contracts.

## Design Summary

### Recommended Shape

Use a single consensus-critical VM at the core boundary:

1. **Outer VM:** PolkaVM-backed RISC-V runtime only.
2. **Legacy contract kind:** current NeoVM bytecode, executed by a privileged `Neo.LegacyVM` contract image running inside the outer RISC-V VM.
3. **Native RISC-V contract kind:** PolkaVM guest bytecode packaged in the normal NEF `Script` field and selected by optional metadata.

This is the lowest-risk way to satisfy all three requirements at once:

- only one VM in Neo core
- existing contracts and tools keep working unchanged
- new RISC-V contracts become first-class citizens

### Rejected Alternatives

- **Keep C# NeoVM for old contracts and add RISC-V only for new ones:** rejected because it leaves two core VMs.
- **Translate legacy NeoVM contracts to RISC-V on deploy or on invoke:** rejected because it changes the deployment model, breaks deterministic parity guarantees, and creates tool incompatibility.
- **Wrap every old contract in a new compatibility container:** rejected because it changes user artifacts and deployment flows.

## Hard Compatibility Invariants

The following must remain unchanged for existing contracts:

- NEF binary format
- manifest schema
- script bytes produced by existing compilers and script builders
- syscall names and hashes
- call flags
- witness verification behavior
- notification and log semantics
- storage layout and iterator behavior
- RPC deployment and invocation surface
- wallet signing and transaction construction
- fee semantics for legacy NeoVM contracts

If any of those change, the design fails.

## Contract Kinds

### 1. Legacy NeoVM Contracts

Legacy contracts are the default.

- Any existing deployed contract with no explicit VM-kind marker is treated as `legacy-neovm`.
- Its stored NEF and manifest remain unchanged.
- On invocation, Neo does **not** execute that bytecode directly in C#.
- Instead, Neo packages the invocation into an internal call to a protocol-owned `Neo.LegacyVM` contract image.

This preserves user-facing behavior while enforcing the single-VM rule internally.

### 2. Native RISC-V Contracts

New RISC-V contracts use the same outer artifact shape:

- standard NEF container
- standard manifest
- `NEF.Script` contains PolkaVM-compatible RISC-V program bytes
- manifest `extra` contains an explicit VM marker, for example:

```json
{
  "vm": "riscv32-polkavm-v1"
}
```

Old tools ignore unknown manifest `extra` fields, so this is backward-compatible. Existing contracts need no change because absence of the field means `legacy-neovm`.

## Core Dispatch Model

Add an internal `ContractVmKind` discriminator in Neo core:

- `LegacyNeoVm`
- `RiscvPolkaVm`

Resolution rules:

1. If manifest `extra.vm == "riscv32-polkavm-v1"`, treat as native RISC-V.
2. Otherwise default to `LegacyNeoVm`.

This choice must happen only in Neo core execution routing. It must not require any user wrapping or RPC changes.

## NeoVM As A Contract

### Protocol-Owned Legacy Contract

`Neo.LegacyVM` should be a protocol-reserved contract image, not a normal user-upgradeable contract.

Properties:

- fixed script hash known to the protocol
- bundled with the node like a native contract image or genesis-owned system package
- executed by the outer RISC-V runtime only
- not directly deployable, replaceable, or deletable by users
- upgraded only by hardfork / protocol version change

### Runtime Behavior

`Neo.LegacyVM` contains the Rust NeoVM interpreter compiled for PolkaVM. It receives:

- target contract script bytes
- initial evaluation stack
- current instruction pointer
- trigger and call flags
- script hash chain / invocation context
- gas-left snapshot
- handles for storage, notifications, logs, iterators, and interop state

It then interprets the exact existing NeoVM bytecode semantics and reports:

- final VM state
- final stack
- fault text
- executed Neo opcode stream for fee accounting
- any requested host interop calls

The key point is that the **same interpreter** used for standalone NeoVM compatibility testing should be the code embedded in `Neo.LegacyVM`. One legacy interpreter implementation, one parity target.

## Execution Flow

### Existing Contract Invocation

1. C# `ApplicationEngine.Create()` always creates the outer RISC-V-backed engine.
2. When a contract is loaded, Neo resolves `ContractVmKind`.
3. For `LegacyNeoVm`, Neo internally invokes `Neo.LegacyVM.RunLegacy`.
4. The outer RISC-V runtime executes that contract image.
5. `Neo.LegacyVM` interprets the target contract’s NeoVM bytecode.
6. All chain-facing effects still go through the same C# host bridge.

### Native RISC-V Contract Invocation

1. Neo resolves `ContractVmKind = RiscvPolkaVm`.
2. The outer RISC-V runtime loads the contract’s `NEF.Script` directly as a PolkaVM guest payload.
3. The contract executes natively under the same host bridge.

### Cross-Kind Calls

The same `System.Contract.Call` dispatcher handles:

- legacy -> legacy
- legacy -> riscv
- riscv -> legacy
- riscv -> riscv

The dispatcher must select the callee path by `ContractVmKind`, not by caller kind.

## Minimal Core Changes

To keep Neo N3 changes small, the design should reuse existing seams:

### Reuse `IApplicationEngineProvider`

- Keep `ApplicationEngine.Create()` as the single public factory.
- Keep provider-based redirection already in place.
- Do not add a second public execution API.

### Keep Existing NEF + Manifest

- No new deployment RPC.
- No new transaction format.
- No new witness format.
- No migration tooling required for legacy contracts.

### Add Only Internal Routing

The new pieces in Neo core should be limited to:

- `ContractVmKind` resolution
- internal dispatch to `Neo.LegacyVM` vs direct RISC-V contract execution
- packaging of the hidden legacy-invocation envelope

Everything else stays behind the current provider/bridge boundary.

## Syscall and State Model

The C# side remains authoritative for:

- storage snapshots
- native contract policy
- notifications and logs
- witness checks
- iterator lifetime
- protocol settings
- trigger context
- final state commits

Both legacy and native RISC-V contracts must see the same syscall namespace and return shapes.

That means:

- do **not** create a second syscall API for RISC-V contracts unless strictly necessary
- do **not** change hash values for existing syscalls
- do **not** fork storage semantics by contract kind

## Fee Model

### Legacy Contracts

Legacy fee semantics must remain bytecode-compatible with today’s NeoVM:

- charge based on interpreted NeoVM opcodes
- keep C# as source of truth for syscall fixed fees and native contract charges
- treat outer RISC-V instruction cost as implementation detail, not user-visible billing

### Native RISC-V Contracts

Native RISC-V contracts can have a RISC-V-specific metering table, but it must be:

- deterministic
- versioned
- independent from host CPU
- surfaced as protocol policy, not hidden implementation cost

The recommended first version is `riscv-metering-v1`, stored as protocol config, not embedded ad hoc in contracts.

## Tooling Impact

### Existing Tooling

Unchanged:

- existing Neo compilers
- script builders
- manifest tooling
- deployment RPCs
- wallet tools
- invoke/testinvoke flows

### New Tooling

Only new RISC-V contract authors need extra tooling:

- RISC-V compiler / packager
- manifest `extra.vm = "riscv32-polkavm-v1"`

This is additive and does not affect legacy users.

## Testing Requirements

Backward compatibility is only credible if validated at three layers:

1. **Standalone interpreter parity**
   - copy and run the `neo-vm` corpus against the Rust legacy interpreter path
2. **Neo core parity**
   - existing `Neo.UnitTests` must stay green with only the outer RISC-V path active
3. **Cross-kind integration**
   - explicit tests for legacy <-> riscv contract calls, notifications, storage, and fault propagation

## Rollout Sequence

### Phase 1

- Outer RISC-V engine becomes default execution path
- direct C# NeoVM path still available only as emergency fallback
- prove parity on tests

### Phase 2

- introduce `ContractVmKind`
- add explicit RISC-V contract packaging support
- add `Neo.LegacyVM` protocol-owned contract image
- route legacy invocations through `Neo.LegacyVM`

### Phase 3

- remove direct C# NeoVM execution path from consensus-critical runtime
- keep pure NeoVM only in tests/benchmarks if still needed

## Success Criteria

The design is complete only when all of the following are true:

- Neo core has one execution VM: outer RISC-V
- all existing contracts run unchanged
- all existing deployment/invocation tools keep working unchanged for legacy contracts
- new RISC-V contracts can be deployed in the same NEF/manifest envelope
- `System.Contract.Call` works across both contract kinds
- legacy fee semantics remain identical
- no user-visible migration is required for old contracts

## Implementation Outline

### Task 1: Add internal VM-kind routing

**Files:**
- Modify: `neo-project/neo/src/Neo/SmartContract/...`

Add `ContractVmKind` resolution with `legacy` as default and explicit manifest-extra opt-in for RISC-V.

### Task 2: Introduce `Neo.LegacyVM` protocol image

**Files:**
- Modify: Rust guest packaging
- Modify: Neo core system-contract/bootstrap registry

Package the Rust NeoVM interpreter as a protocol-owned RISC-V contract image and make legacy execution route through it internally.

### Task 3: Add native RISC-V contract packaging

**Files:**
- Modify: RISC-V compiler/packager
- Modify: deployment validation path

Accept PolkaVM guest blobs inside standard NEF `Script` and validate explicit VM metadata.

### Task 4: Prove backward compatibility

**Files:**
- Modify: Neo unit tests
- Modify: standalone VM compatibility harness
- Modify: benchmarks

Require old tests to pass unchanged and add bridge/cross-kind parity tests plus benchmarks.
