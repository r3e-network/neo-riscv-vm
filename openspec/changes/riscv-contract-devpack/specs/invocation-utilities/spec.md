## ADDED Requirements

### Requirement: Invoke deployed contracts

The system SHALL provide utilities to invoke deployed RISC-V contracts.

#### Scenario: Call contract method

- **WHEN** user invokes contract method
- **THEN** system SHALL construct invocation transaction with parameters

### Requirement: Query contract state

The system SHALL provide utilities to read contract storage.

#### Scenario: Read contract storage

- **WHEN** user queries contract state
- **THEN** system SHALL return storage values without transaction cost
