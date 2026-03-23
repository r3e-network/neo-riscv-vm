## ADDED Requirements

### Requirement: Provide Neo N3 syscall bindings

The devpack SHALL provide Rust bindings for all Neo N3 syscalls.

#### Scenario: Call native contract

- **WHEN** contract calls System.Contract.Call
- **THEN** system SHALL invoke native contract via syscall

### Requirement: Standard types library

The devpack SHALL provide Neo types (Hash160, Hash256, PublicKey).

#### Scenario: Use Hash160 type

- **WHEN** contract uses Hash160
- **THEN** system SHALL provide 20-byte address type
