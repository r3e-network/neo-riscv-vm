#!/usr/bin/env python3
"""Collect reference RPC application logs into a bucketed mainnet oracle corpus."""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Iterable


def rpc_call(url: str, method: str, params: list[Any], timeout: float = 30.0) -> Any:
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


def bucket_execution(execution: dict[str, Any]) -> str:
    vmstate = str(execution.get("vmstate") or "unknown")
    exception = execution.get("exception") or "none"
    event_names = sorted(
        {
            str(notification.get("eventname"))
            for notification in execution.get("notifications") or []
            if notification.get("eventname") is not None
        }
    )
    events = ",".join(event_names) if event_names else "none"
    return f"vmstate={vmstate};exception={exception};events={events}"


def iter_block_transactions(block: dict[str, Any]) -> Iterable[str]:
    for tx in block.get("tx") or []:
        if isinstance(tx, dict) and tx.get("hash"):
            yield str(tx["hash"])
        elif isinstance(tx, str):
            yield tx


def collect_range(
    reference_rpc: str,
    start: int,
    end: int,
    *,
    timeout: float = 30.0,
    sleep_seconds: float = 0.0,
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for height in range(start, end + 1):
        block = rpc_call(reference_rpc, "getblock", [height, 1], timeout=timeout)
        if not isinstance(block, dict):
            continue
        for tx_hash in iter_block_transactions(block):
            app_log = rpc_call(reference_rpc, "getapplicationlog", [tx_hash], timeout=timeout)
            for execution in (app_log or {}).get("executions") or []:
                notifications = execution.get("notifications") or []
                records.append(
                    {
                        "block": height,
                        "tx": tx_hash,
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
                        "bucket": bucket_execution(execution),
                    }
                )
            if sleep_seconds > 0:
                time.sleep(sleep_seconds)
    return records


def write_jsonl(records: Iterable[dict[str, Any]], output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as f:
        for record in records:
            f.write(json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-rpc", default="http://seed1.neo.org:10332")
    parser.add_argument("--start", type=int, required=True)
    parser.add_argument("--end", type=int, required=True)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--sleep", type=float, default=0.0, help="sleep between tx applicationlog requests")
    parser.add_argument("--output", default="tests/corpus/mainnet-oracle/applicationlogs.jsonl")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.end < args.start:
        print("--end must be >= --start", file=sys.stderr)
        return 64
    records = collect_range(
        args.reference_rpc,
        args.start,
        args.end,
        timeout=args.timeout,
        sleep_seconds=args.sleep,
    )
    write_jsonl(records, Path(args.output))
    buckets = sorted({record["bucket"] for record in records})
    print(f"mainnet_oracle_corpus blocks={args.start}..{args.end} records={len(records)} buckets={len(buckets)} output={args.output}")
    for bucket in buckets[:40]:
        print(f"bucket {bucket}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
