## ADDED Requirements

### Requirement: Plugin Loading Validation

The system SHALL verify Neo.Riscv.Adapter plugin loads correctly in neo-riscv-core.

#### Scenario: Plugin discovery

- **WHEN** neo-riscv-core starts with Neo.Riscv.Adapter in Plugins directory
- **THEN** system loads plugin and registers RISC-V VM provider

#### Scenario: FFI library resolution

- **WHEN** plugin initializes
- **THEN** system locates and loads libneo_riscv_host.so without errors

### Requirement: Contract Execution Integration

The system SHALL validate end-to-end contract execution through all three repositories.

#### Scenario: Deploy RISC-V contract via neo-riscv-node

- **WHEN** deploying contract with RISC-V bytecode through RPC
- **THEN** system stores contract and returns deployment transaction

#### Scenario: Invoke RISC-V contract

- **WHEN** invoking deployed RISC-V contract via RPC
- **THEN** system executes in RISC-V VM and returns correct result

### Requirement: Build Dependency Chain

The system SHALL validate correct build order across repositories.

#### Scenario: Clean build from scratch

- **WHEN** building all three repos in dependency order
- **THEN** system completes without errors: vm → core → node

#### Scenario: Incremental rebuild

- **WHEN** modifying neo-riscv-vm and rebuilding downstream
- **THEN** system detects changes and rebuilds core and node

### Requirement: Consensus Behavior

The system SHALL validate RISC-V contracts participate correctly in consensus.

#### Scenario: Block validation with RISC-V contract

- **WHEN** block contains RISC-V contract invocation
- **THEN** all nodes reach consensus on execution result

#### Scenario: Gas consumption in consensus

- **WHEN** validating block with RISC-V contract
- **THEN** all nodes agree on gas consumed
