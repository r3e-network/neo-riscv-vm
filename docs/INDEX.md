# Documentation Index

**Version:** 1.1  
**Last Updated:** 2026-03-24  
**Status:** Current

## Start Here

- [Current Status](./CURRENT_STATUS.md)
- [Final Validation Report](./FINAL_VALIDATION_REPORT.md)
- [Testing Guide](./TESTING.md)
- [Architecture](./ARCHITECTURE.md)
- [API Reference](./API_REFERENCE.md)

## Recommended Reading By Role

### Operators

1. [Current Status](./CURRENT_STATUS.md)
2. [DEPLOYMENT.md](./DEPLOYMENT.md)
3. [TESTING.md](./TESTING.md)
4. [troubleshooting.md](./troubleshooting.md)

### Developers

1. [Architecture](./ARCHITECTURE.md)
2. [API Reference](./API_REFERENCE.md)
3. [Testing Guide](./TESTING.md)
4. [NEP-RISC-V-VM](./NEP-RISC-V-VM.md)

### Stakeholders

1. [Current Status](./CURRENT_STATUS.md)
2. [Final Validation Report](./FINAL_VALIDATION_REPORT.md)
3. [README](../README.md)

## Canonical Current Documents

- [Current Status](./CURRENT_STATUS.md): exact architecture and caveats
- [Final Validation Report](./FINAL_VALIDATION_REPORT.md): fresh evidence from the committed matrix
- [Testing Guide](./TESTING.md): commands and suite boundaries
- [Architecture](./ARCHITECTURE.md): system design
- [NEP-RISC-V-VM](./NEP-RISC-V-VM.md): technical specification

## Historical Documents

These are retained for context, not as the exact current implementation narrative:

- [Historical Zero-Change Target](./ACHIEVED_ZERO_CHANGE.md)
- [Historical Zero-Change Architecture](./ZERO_CHANGE_ARCHITECTURE.md)

## Current Validation Snapshot

- VM workspace tests: `311`
- JSON compatibility corpus: `161` files
- Core matrix: `1171`
- Node matrix: `477`
- Extra smoke: VM E2E, FFI resolution, `neo-cli` smoke

Canonical command:

```bash
./scripts/cross-repo-test.sh
```
