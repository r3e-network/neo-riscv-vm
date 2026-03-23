# Monitoring Configuration

## Structured Logging

Add to your application:

```rust
use tracing::{info, error, warn};
use tracing_subscriber;

// Initialize at startup
tracing_subscriber::fmt::init();

// Usage in code
info!(contract_hash = %hash, "Executing contract");
error!(error = %e, "Execution failed");
```

## Log Levels

- `ERROR`: Execution failures, FFI panics
- `WARN`: Performance degradation, gas warnings
- `INFO`: Contract execution start/end
- `DEBUG`: Opcode-level tracing
- `TRACE`: Stack state dumps

## Production Configuration

```bash
# Environment variables
export RUST_LOG=neo_riscv_host=info,neo_riscv_guest=warn
export RUST_LOG_STYLE=always
```

## Log Aggregation

Forward logs to your monitoring system:

- Elasticsearch + Kibana
- Grafana Loki
- CloudWatch Logs
