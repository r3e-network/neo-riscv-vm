## Why

Previous validation infrastructure established the shape of tests, benchmarks, and monitoring. Production deployment requires full execution validation, performance baselines, and operational observability.

## What Changes

- Convert opcode validation structure into real NeoVM script execution tests
- Implement actual performance benchmarks with baseline measurements against native NeoVM
- Add production monitoring with Prometheus metrics and structured logging
- Write essential API documentation (rustdoc + XML docs)
- Create deployment runbook with troubleshooting guides

## Capabilities

### New Capabilities

- `opcode-execution-tests`: Comprehensive opcode validation with real NeoVM scripts
- `performance-benchmarks`: Criterion benchmarks with 2x native NeoVM threshold validation
- `production-monitoring`: Prometheus metrics, structured logging, health checks
- `api-documentation`: Complete rustdoc and C# XML documentation
- `deployment-operations`: Runbooks, troubleshooting guides, configuration templates

### Modified Capabilities

<!-- No existing spec requirements are changing -->

## Impact

- **Rust crates**: All 4 workspace crates (abi, guest, guest-module, host)
- **C# adapter**: Neo.Riscv.Adapter plugin
- **Test infrastructure**: tests/opcode_matrix/, benches/
- **Documentation**: README.md, rustdoc comments, C# XML docs
- **Operations**: New monitoring endpoints, logging configuration

## Implementation Status Note (2026-03-24)

The final committed workspace externalized the `Neo.SmartContract.RiscV` bridge/provider code into `neo-riscv-vm/compat/Neo.Riscv.Adapter`. The current implementation is validated and production-ready for the integrated three-repo workspace, but it is no longer a literal zero-change core/node shape.
