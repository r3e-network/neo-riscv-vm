# Getting Started with Neo RISC-V Contracts

## Prerequisites

- Rust toolchain (stable)
- PolkaVM tools

## Quick Start

1. Create contract:

```bash
cargo new --lib my-contract
```

2. Build:

```bash
./scripts/build-contract.sh
```

3. Deploy:

```bash
./scripts/deploy-contract.sh contract.nef
```
