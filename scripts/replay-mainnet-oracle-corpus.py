#!/usr/bin/env python3
"""Replay a stored mainnet applicationlog oracle corpus against a local RPC node."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Iterable


class ReplayResult:
    def __init__(self, checked: int, mismatches: list[dict[str, Any]], missing: list[dict[str, Any]]) -> None:
        self.checked = checked
        self.mismatches = mismatches
        self.missing = missing


def rpc_call(url: str, method: str, params: list[Any], timeout: float = 20.0) -> Any:
    payload = json.dumps({"jsonrpc": "2.0", "method": method, "params": params, "id": 1}).encode()
    request = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = json.loads(response.read().decode())
    except urllib.error.URLError as exc:
        raise RuntimeError(f"RPC {method} failed: {exc}") from exc
    if body.get("error"):
        raise RuntimeError(f"RPC {method} returned error: {body['error']}")
    return body.get("result")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            if not isinstance(record, dict):
                raise ValueError(f"{path}:{line_no}: expected object")
            records.append(record)
    return records


def execution_summary(execution: dict[str, Any]) -> dict[str, Any]:
    notifications = execution.get("notifications") or []
    return {
        "trigger": execution.get("trigger"),
        "vmstate": execution.get("vmstate"),
        "exception": execution.get("exception"),
        "event_names": sorted(
            {
                str(notification.get("eventname"))
                for notification in notifications
                if notification.get("eventname") is not None
            }
        ),
        "notification_count": len(notifications),
        "stack": execution.get("stack"),
    }


def select_execution(application_log: dict[str, Any] | None, record: dict[str, Any]) -> dict[str, Any] | None:
    executions = (application_log or {}).get("executions") or []
    execution_index = record.get("execution_index")
    if isinstance(execution_index, int) and 0 <= execution_index < len(executions):
        return executions[execution_index]
    for execution in executions:
        if execution.get("trigger") == record.get("trigger"):
            return execution
    return executions[0] if executions else None


def compare_record(record: dict[str, Any], local_execution: dict[str, Any]) -> list[dict[str, Any]]:
    local = execution_summary(local_execution)
    mismatches: list[dict[str, Any]] = []
    for field in ("trigger", "vmstate", "exception", "event_names", "notification_count", "stack"):
        expected = record.get(field)
        actual = local.get(field)
        if expected != actual:
            mismatches.append(
                {
                    "tx": record.get("tx"),
                    "block": record.get("block"),
                    "execution_index": record.get("execution_index"),
                    "field": field,
                    "expected": expected,
                    "actual": actual,
                }
            )
    return mismatches


def replay_records(
    records: Iterable[dict[str, Any]],
    local_rpc: str,
    *,
    timeout: float = 20.0,
    max_records: int | None = None,
) -> ReplayResult:
    checked = 0
    mismatches: list[dict[str, Any]] = []
    missing: list[dict[str, Any]] = []
    app_log_cache: dict[str, Any] = {}

    for record in records:
        if max_records is not None and checked >= max_records:
            break
        tx_hash = str(record.get("tx") or "")
        if not tx_hash:
            missing.append({"record": record, "reason": "missing_tx"})
            continue
        if tx_hash not in app_log_cache:
            try:
                app_log_cache[tx_hash] = rpc_call(local_rpc, "getapplicationlog", [tx_hash], timeout=timeout)
            except Exception as exc:
                missing.append({"tx": tx_hash, "block": record.get("block"), "reason": str(exc)})
                continue

        execution = select_execution(app_log_cache[tx_hash], record)
        if execution is None:
            missing.append({"tx": tx_hash, "block": record.get("block"), "reason": "local_applicationlog_missing"})
            continue

        checked += 1
        mismatches.extend(compare_record(record, execution))

    return ReplayResult(checked=checked, mismatches=mismatches, missing=missing)


def write_jsonl(records: Iterable[dict[str, Any]], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as f:
        for record in records:
            f.write(json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", default="mainnet-validation/oracle-corpus/applicationlogs.jsonl")
    parser.add_argument("--local-rpc", default="http://127.0.0.1:10332")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--max-records", type=int)
    parser.add_argument("--output", default="mainnet-validation/oracle-corpus/replay-mismatches.jsonl")
    parser.add_argument("--fail-on-mismatch", action="store_true")
    parser.add_argument("--fail-on-missing-local", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    corpus_path = Path(args.corpus)
    if not corpus_path.exists():
        print(f"corpus not found: {corpus_path}", file=sys.stderr)
        return 2

    result = replay_records(
        read_jsonl(corpus_path),
        args.local_rpc,
        timeout=args.timeout,
        max_records=args.max_records,
    )
    write_jsonl(result.mismatches + result.missing, Path(args.output))
    print(
        "mainnet_oracle_replay "
        f"checked={result.checked} mismatches={len(result.mismatches)} "
        f"missing={len(result.missing)} output={args.output}"
    )
    for mismatch in result.mismatches[:20]:
        print(
            "MISMATCH "
            f"block={mismatch.get('block')} tx={mismatch.get('tx')} field={mismatch.get('field')} "
            f"expected={mismatch.get('expected')} actual={mismatch.get('actual')}"
        )
    if args.fail_on_mismatch and result.mismatches:
        return 1
    if args.fail_on_missing_local and result.missing:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
