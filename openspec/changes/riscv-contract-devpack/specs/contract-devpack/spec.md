## ADDED Requirements

### Requirement: Provide Neo N3 syscall bindings

The devpack SHALL provide Rust bindings for Neo N3 syscalls by forwarding to the existing C# Neo implementation.

#### Scenario: Call native contract

- **WHEN** contract calls System.Contract.Call
- **THEN** system SHALL invoke native contract via syscall

### Requirement: Reuse existing syscall and native-contract implementations

The system SHALL treat the existing C# Neo engine as the source of truth for syscall and native-contract behavior.

#### Scenario: Syscall execution

- **WHEN** a RISC-V contract invokes a syscall
- **THEN** the request SHALL be forwarded to the existing C# Neo syscall implementation
- **AND** the system SHALL NOT execute a separate Rust/RISC-V reimplementation of that syscall

#### Scenario: Native contract execution

- **WHEN** a RISC-V contract calls a native contract
- **THEN** the request SHALL be routed to the existing C# Neo native-contract implementation
- **AND** the system SHALL NOT maintain a separate Rust/RISC-V implementation of that native contract

### Requirement: Standard types library

The devpack SHALL provide Neo types (Hash160, Hash256, PublicKey).

#### Scenario: Use Hash160 type

- **WHEN** contract uses Hash160
- **THEN** system SHALL provide 20-byte address type
