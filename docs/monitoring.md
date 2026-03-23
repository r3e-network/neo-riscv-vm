# Monitoring Configuration

## Prometheus Metrics (Future)

When HTTP server is added, expose metrics at `/metrics`:

```
# Execution counters
neo_riscv_executions_total{status="success"} 1234
neo_riscv_executions_total{status="fault"} 56

# Gas consumption
neo_riscv_gas_consumed_pico_total 9876543210

# Execution time
neo_riscv_execution_duration_seconds_bucket{le="0.1"} 1000
```

## Health Check (Future)

Endpoint: `GET /health`

Response:

```json
{
    "status": "healthy",
    "version": "0.1.0",
    "uptime_seconds": 3600
}
```

## Current Monitoring

Use tracing logs (see logging.md) until HTTP endpoints are implemented.
