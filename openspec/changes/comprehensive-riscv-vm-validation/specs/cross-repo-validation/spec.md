## ADDED Requirements

### Requirement: Cross-Repo Test Orchestration
The system SHALL orchestrate integration tests across the `neo-riscv-vm`, `neo-riscv-core`, and `neo-riscv-node` repositories.

#### Scenario: Orchestration execution
- **WHEN** a change is pushed or a scheduled run is triggered
- **THEN** tests across all three repositories are executed using the latest VM build

### Requirement: Version Pinning
The orchestration framework SHALL support pinning repository versions or commit hashes to ensure reproducible cross-repo test runs.

#### Scenario: Run with pinned versions
- **WHEN** specific commit hashes are provided in the test configuration
- **THEN** the orchestration framework checks out and tests against those exact versions

### Requirement: Cross-Repo CI Reporting
The CI pipeline SHALL aggregate and report test results from all involved repositories in a unified manner.

#### Scenario: CI result aggregation
- **WHEN** the cross-repo test suite completes
- **THEN** a unified report is generated showing passes/fails across `vm`, `core`, and `node` tests
