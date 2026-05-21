# Documentation Index

**Version:** 1.2
**Last Updated:** 2026-05-21
**Status:** Current

## Start Here

- [Current Status](./CURRENT_STATUS.md)
- [Testing Guide](./TESTING.md)
- [Architecture](./ARCHITECTURE.md)
- [Visual Diagrams](./diagrams/README.md)
- [API Reference](./API_REFERENCE.md)

## Recommended Reading By Role

### Operators

1. [Current Status](./CURRENT_STATUS.md)
2. [Deployment Guide](./DEPLOYMENT.md)
3. [Testing Guide](./TESTING.md)
4. [Troubleshooting](./troubleshooting.md)

### Developers

1. [Architecture](./ARCHITECTURE.md)
2. [Visual Diagrams](./diagrams/README.md)
3. [API Reference](./API_REFERENCE.md)
4. [Testing Guide](./TESTING.md)
5. [NEP RISC-V VM](./NEP-RISC-V-VM.md)

### Stakeholders

1. [Current Status](./CURRENT_STATUS.md)
2. [Visual Diagrams](./diagrams/README.md)
3. [README](../README.md)

## Canonical Current Documents

- [Current Status](./CURRENT_STATUS.md): exact architecture and caveats
- [Testing Guide](./TESTING.md): commands and suite boundaries
- [Architecture](./ARCHITECTURE.md): system design
- [Visual Diagrams](./diagrams/README.md): architecture, data-flow, workflow, and validation diagrams
- [NEP RISC-V VM](./NEP-RISC-V-VM.md): technical specification

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
