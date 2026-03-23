## ADDED Requirements

### Requirement: Arithmetic opcode validation

The system SHALL validate all arithmetic opcodes (ADD, SUB, MUL, DIV, MOD) with real NeoVM script execution including overflow/underflow edge cases.

#### Scenario: ADD with overflow

- **WHEN** executing ADD with i64::MAX values
- **THEN** system SHALL match NeoVM overflow behavior

#### Scenario: DIV by zero

- **WHEN** executing DIV with zero divisor
- **THEN** system SHALL FAULT matching NeoVM

### Requirement: Stack opcode validation

The system SHALL validate all stack opcodes (PUSH, POP, DUP, SWAP, ROT, REVERSE) with real script execution.

#### Scenario: POP on empty stack

- **WHEN** executing POP with empty stack
- **THEN** system SHALL FAULT

#### Scenario: DUP on empty stack

- **WHEN** executing DUP with empty stack
- **THEN** system SHALL FAULT

### Requirement: Control flow validation

The system SHALL validate all control flow opcodes (JMP, JMPIF, CALL, RET, TRY, ENDTRY) with real scripts.

#### Scenario: TRY-ENDTRY exception handling

- **WHEN** executing TRY block with FAULT inside
- **THEN** system SHALL catch and continue execution

### Requirement: Type conversion validation

The system SHALL validate all type opcodes (CONVERT, ISTYPE) with real scripts covering all NeoVM types.

#### Scenario: Buffer to Boolean conversion

- **WHEN** converting non-empty Buffer to Boolean
- **THEN** system SHALL return true
