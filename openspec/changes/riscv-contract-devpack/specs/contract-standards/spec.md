## ADDED Requirements

### Requirement: Define contract manifest schema

The system SHALL define manifest format for RISC-V contracts compatible with Neo N3.

#### Scenario: Deploy contract with manifest

- **WHEN** developer deploys RISC-V contract
- **THEN** system SHALL validate manifest schema (name, groups, features, abi, permissions, trusts)

### Requirement: Define contract entry points

The system SHALL define standard entry points for RISC-V contracts.

#### Scenario: Invoke contract method

- **WHEN** system invokes contract
- **THEN** contract SHALL expose standard entry point function
