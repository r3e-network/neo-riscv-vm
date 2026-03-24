# Deployment Guide

**Version:** 1.0  
**Last Updated:** 2026-03-24  
**Status:** Production Ready

---

## Quick Start (5 Minutes)

### 1. Build

```bash
cd neo-riscv-vm

# Build release version
cargo build -p neo-riscv-host --release

# Package plugin
./scripts/package-adapter-plugin.sh
```

### 2. Deploy

```bash
# Copy to your Neo node
cp -r dist/Plugins/Neo.Riscv.Adapter /path/to/neo-cli/Plugins/

# Verify files
ls /path/to/neo-cli/Plugins/Neo.Riscv.Adapter/
# Neo.Riscv.Adapter.dll
# libneo_riscv_host.so
```

### 3. Run

```bash
cd /path/to/neo-cli
./neo-cli

# Look for in logs:
# "RISC-V adapter initialized, provider registered"
```

### 4. Verify

```bash
# Local VM verification
./scripts/run-all-tests.sh quick

# Full integrated workspace validation
./scripts/cross-repo-test.sh
```

---

## Deployment Options

### Option A: Plugin Directory (Recommended)

**Structure:**
```
neo-cli/
├── neo-cli.dll
├── config.json
└── Plugins/
    └── Neo.Riscv.Adapter/
        ├── Neo.Riscv.Adapter.dll      # C# bridge (62KB)
        └── libneo_riscv_host.so       # RISC-V runtime (2.5MB)
```

**Pros:**
- Zero configuration
- Auto-discovery by Neo
- Easy to enable/disable
- Works with any Neo node

**Cons:**
- None

### Option B: Environment Variable

**For development or custom paths:**

```bash
export NEO_RISCV_HOST_LIB=/custom/path/libneo_riscv_host.so
./neo-cli
```

**Pros:**
- Flexible library location
- Multiple versions possible
- Good for testing

**Cons:**
- Requires environment setup
- Not persistent across sessions

### Option C: System-Wide Install

**For production servers:**

```bash
# Install library system-wide
sudo cp target/release/libneo_riscv_host.so /usr/local/lib/
sudo ldconfig

# Run without environment variable
./neo-cli
```

**Pros:**
- Single copy for multiple nodes
- Managed by system package manager

**Cons:**
- Requires root access
- Affects all users

### Option D: Docker

**Dockerfile:**
```dockerfile
FROM neo-cli:latest

# Copy plugin
COPY dist/Plugins/Neo.Riscv.Adapter /app/Plugins/Neo.Riscv.Adapter

# Set library path
ENV NEO_RISCV_HOST_LIB=/app/Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so

EXPOSE 10332 10333

ENTRYPOINT ["./neo-cli"]
```

**Build and run:**
```bash
docker build -t neo-riscv .
docker run -p 10332:10332 neo-riscv
```

---

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux x86_64 | ✅ Supported | Primary platform |
| Linux ARM64 | ⚠️ Experimental | Build from source |
| macOS | ❌ Not supported | Requires porting |
| Windows | ❌ Not supported | Requires porting |

### System Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | x86_64 | x86_64 (2+ cores) |
| RAM | 512MB | 1GB+ |
| Disk | 100MB | 1GB+ |
| OS | Linux 5.4+ | Ubuntu 22.04 LTS |

---

## Installation Methods

### Method 1: Binary Release (Recommended)

**Download pre-built release:**
```bash
wget https://github.com/neo-riscv/vm/releases/download/v1.0/neo-riscv-vm-v1.0-linux-x64.tar.gz
tar xzf neo-riscv-vm-v1.0-linux-x64.tar.gz
cp -r Plugins/* /path/to/neo-cli/Plugins/
```

### Method 2: Build from Source

**Requirements:**
- Rust 1.70+
- .NET 10.0 SDK (for tests)

**Build:**
```bash
git clone https://github.com/neo-riscv/vm.git
cd neo-riscv-vm
cargo build -p neo-riscv-host --release
./scripts/package-adapter-plugin.sh
```

### Method 3: Package Manager

**Future:** apt/yum packages
```bash
# Coming soon
sudo apt install neo-riscv-adapter
```

---

## Configuration

### No Extra Runtime Configuration Required

The packaged adapter bundle is designed to work from the standard Neo `Plugins/Neo.Riscv.Adapter/` location without extra runtime flags.

### Optional Settings

**Environment Variables:**

```bash
# Custom library path
export NEO_RISCV_HOST_LIB=/custom/path/libneo_riscv_host.so

# Enable detailed tracing
export NEO_RISCV_TRACE_ENGINE=1

# JSON test mode
export NEO_RISCV_VM_JSON_MODE=full

# Verbose output
export NEO_RISCV_VM_JSON_VERBOSE=1
```

**Neo config.json:**

No changes needed. The plugin auto-registers.

---

## Verification

### Post-Deployment Checks

```bash
# 1. Check plugin loaded
grep -i "risc" /path/to/neo-cli/Logs/*.log
# Expected: "RISC-V adapter initialized"

# 2. Run quick test suite
./scripts/run-all-tests.sh quick

# 3. Check memory usage
ps aux | grep neo-cli
# Expected: +256MB RSS

# 4. Test contract execution
curl -X POST http://localhost:10332 \
  -d '{"jsonrpc":"2.0","method":"invokefunction","params":["0x...","symbol",[]],"id":1}'
```

### Health Check Script

```bash
#!/bin/bash
# health-check.sh

echo "Checking RISC-V VM health..."

# Check process running
if ! pgrep -x "neo-cli" > /dev/null; then
    echo "❌ neo-cli not running"
    exit 1
fi
echo "✅ neo-cli running"

# Check plugin loaded
if ! grep -q "RISC-V adapter initialized" /path/to/neo-cli/Logs/*.log 2>/dev/null; then
    echo "❌ RISC-V adapter not loaded"
    exit 1
fi
echo "✅ RISC-V adapter loaded"

# Check library exists
if [ ! -f "/path/to/neo-cli/Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so" ]; then
    echo "❌ Native library not found"
    exit 1
fi
echo "✅ Native library found"

# Run tests
if ! ./scripts/run-all-tests.sh quick > /dev/null 2>&1; then
    echo "❌ Tests failing"
    exit 1
fi
echo "✅ All tests passing"

echo ""
echo "🎉 RISC-V VM is healthy!"
```

---

## Monitoring

### Key Metrics

| Metric | Expected | Alert If |
|--------|----------|----------|
| Memory (RSS) | 256-300MB | >500MB |
| CPU (idle) | <1% | >10% |
| CPU (execution) | Varies | >100ms/op |
| Test pass rate | 100% | <100% |

### Logging

**Log Location:**
```
/path/to/neo-cli/Logs/
├── Neo-YYYY-MM-DD.log       # Neo core logs
└── neo-riscv-*.log          # RISC-V specific (if enabled)
```

**Enable RISC-V Tracing:**
```bash
export NEO_RISCV_TRACE_ENGINE=1
./neo-cli
```

**Log Levels:**
- ERROR: VM faults, out of gas
- WARN: Performance issues
- INFO: General operations
- DEBUG: Detailed tracing

### Metrics Export

**Prometheus (future):**
```yaml
# metrics.yml
riscv_vm_executions_total
counter: Total contract executions

riscv_vm_execution_duration_seconds
histogram: Execution duration

riscv_vm_gas_consumed_total
counter: Total gas consumed

riscv_vm_memory_bytes
gauge: Current memory usage
```

---

## Upgrading

### Minor Updates (Bug Fixes)

```bash
# 1. Backup current plugin
cp -r Plugins/Neo.Riscv.Adapter Plugins/Neo.Riscv.Adapter.backup

# 2. Stop node
pkill neo-cli

# 3. Deploy new version
cp -r new-release/Plugins/Neo.Riscv.Adapter Plugins/

# 4. Start node
./neo-cli

# 5. Verify
./scripts/run-all-tests.sh quick
```

### Major Updates (Breaking Changes)

```bash
# 1. Full backup
mkdir backup-$(date +%Y%m%d)
cp -r Plugins backup-$(date +%Y%m%d)/
cp config.json backup-$(date +%Y%m%d)/

# 2. Test on staging
# ...

# 3. Rolling deployment
# Deploy to one node, verify, then others
```

---

## Rollback

### Emergency Rollback

```bash
# Stop node
pkill neo-cli

# Disable plugin
mv Plugins/Neo.Riscv.Adapter Plugins/Neo.Riscv.Adapter.disabled

# Start node (falls back to NeoVM)
./neo-cli
```

### Verified Rollback

```bash
# 1. Run full test suite on NeoVM
unset NEO_RISCV_HOST_LIB
./scripts/run-all-tests.sh full

# 2. If tests pass, proceed
# 3. If tests fail, investigate before rollback
```

---

## Troubleshooting

### Common Issues

#### Issue: "RISC-V adapter not found"

**Symptoms:**
- Logs don't show "RISC-V adapter initialized"
- Tests fail with "library not found"

**Diagnosis:**
```bash
ls -la Plugins/Neo.Riscv.Adapter/
# Should show:
# - Neo.Riscv.Adapter.dll
# - libneo_riscv_host.so
```

**Solution:**
```bash
# Reinstall plugin
./scripts/package-adapter-plugin.sh
cp -r dist/Plugins/Neo.Riscv.Adapter Plugins/
```

#### Issue: "Native library not found"

**Symptoms:**
- `DllNotFoundException`
- Tests skip with "inconclusive"

**Diagnosis:**
```bash
# Check library exists
ls Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so

# Check architecture
file Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so
# Expected: ELF 64-bit LSB shared object, x86-64

# Check dependencies
ldd Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so
```

**Solution:**
```bash
# Rebuild for your architecture
cargo build -p neo-riscv-host --release

# Or set explicit path
export NEO_RISCV_HOST_LIB=/absolute/path/libneo_riscv_host.so
```

#### Issue: High memory usage

**Symptoms:**
- RSS >500MB
- OOM kills

**Diagnosis:**
```bash
# Check memory usage
ps aux | grep neo-cli

# Check for leaks
valgrind --leak-check=full ./neo-cli
```

**Solution:**
- Expected: +256MB for guest arena
- If significantly higher: report bug

#### Issue: Slow performance

**Symptoms:**
- Operations take >100µs
- High CPU usage

**Diagnosis:**
```bash
# Run benchmarks
cargo bench -p neo-riscv-host

# Check if debug build
file target/debug/libneo_riscv_host.so
# Should use release build
```

**Solution:**
```bash
# Use release build
cargo build -p neo-riscv-host --release

# Enable optimizations (see OPTIMIZATION_PLAN.md)
```

---

## Best Practices

### Production Deployment

1. **Test on staging first**
   ```bash
   ./scripts/run-all-tests.sh full
   ```

2. **Deploy to single node**
   - Monitor for 24 hours
   - Check logs for errors

3. **Rolling deployment**
   - Deploy to consensus nodes one by one
   - Verify each before proceeding

4. **Monitor continuously**
   - Memory usage
   - Error rates
   - Performance metrics

### Security

1. **File permissions**
   ```bash
   chmod 755 Plugins/Neo.Riscv.Adapter/
   chmod 644 Plugins/Neo.Riscv.Adapter/*
   ```

2. **Library integrity**
   ```bash
   # Verify checksum
   sha256sum libneo_riscv_host.so
   ```

3. **Audit logs**
   - Review logs regularly
   - Alert on errors

### Backup

```bash
# Daily backup
#!/bin/bash
DATE=$(date +%Y%m%d)
tar czf backup-$DATE.tar.gz \
  Plugins/Neo.Riscv.Adapter/ \
  config.json \
  Logs/
```

---

## Support

### Resources

- Documentation: [docs/](./)
- Issues: [GitHub Issues](../../issues)
- Discussions: [GitHub Discussions](../../discussions)

### Debug Information

When reporting issues, include:

```bash
# System info
uname -a
ldd --version
dotnet --version
cargo --version

# Build info
file Plugins/Neo.Riscv.Adapter/libneo_riscv_host.so

# Test results
./scripts/run-all-tests.sh full 2>&1 | tail -50

# Logs
tail -100 Logs/Neo-*.log
```

---

## Summary

| Aspect | Recommendation |
|--------|----------------|
| Deployment | Plugin directory (Option A) |
| Verification | Run `./scripts/run-all-tests.sh quick` |
| Monitoring | Memory + logs |
| Rollback | Rename plugin directory |
| Updates | Test on staging first |
