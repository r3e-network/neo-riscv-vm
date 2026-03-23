# Deployment Runbook

## Prerequisites

- Rust 1.70+
- C# .NET 8.0+
- Neo N3 node (v3.9.1+)

## Build Process

### 1. Build Rust Components

```bash
cd ~/git/neo-riscv-vm
cargo build --release
```

Output: `target/release/libneo_riscv_host.so`

### 2. Build C# Adapter

```bash
cd ~/git/neo-riscv-core
dotnet build -c Release src/Neo.Riscv.Adapter
```

### 3. Build Neo Node

```bash
cd ~/git/neo-riscv-node
dotnet build -c Release
```

## Deployment

1. Copy `libneo_riscv_host.so` to Neo node directory
2. Copy `Neo.Riscv.Adapter.dll` to plugins directory
3. Update `config.json` to enable RiscV plugin
4. Start node: `dotnet neo-cli.dll`

## Verification

```bash
# Check plugin loaded
curl http://localhost:10332 -d '{"jsonrpc":"2.0","method":"listplugins","params":[],"id":1}'

# Run test contract
cargo test --release --workspace
```

## Rollback

1. Stop node
2. Remove RiscV plugin from config
3. Restart with previous version
