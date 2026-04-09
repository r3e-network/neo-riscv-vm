# Deployment Runbook

## Prerequisites

- Rust stable toolchain
- Rust nightly toolchain for PolkaVM contract compilation/fuzzing
- .NET 10 SDK
- Neo N3 node (v3.9.1+)
- `polkatool` if you need to compile example or production RISC-V contracts

## Build Process

### 1. Package the adapter bundle

```bash
cd ~/git/neo-riscv/neo-riscv-vm
./scripts/package-adapter-plugin.sh
```

Output: `dist/Plugins/Neo.Riscv.Adapter/`

### 2. Compile a RISC-V contract blob

```bash
cd ~/git/neo-riscv/neo-riscv-vm
./scripts/compile-riscv-contract.sh examples/counter examples/counter/target/counter.polkavm
```

### 3. Build Neo node / CLI

```bash
cd ~/git/neo-riscv/neo-riscv-node
dotnet build -c Release
```

## Deployment

1. Copy `neo-riscv-vm/dist/Plugins` next to your `neo-cli` binaries, as printed by `./scripts/package-adapter-plugin.sh`.
2. Update `config.json` to enable the RISC-V adapter plugin.
3. Start node: `dotnet neo-cli.dll`
4. Package and deploy your `.polkavm` contract blob through the normal Neo deployment flow.

## Verification

```bash
cd ~/git/neo-riscv/neo-riscv-vm
./scripts/test-ffi-resolution.sh
./tests/e2e/run-all.sh
./scripts/cross-repo-test.sh
```

## Rollback

1. Stop node
2. Remove RiscV plugin from config
3. Restart with previous version
