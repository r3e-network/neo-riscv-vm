#!/usr/bin/env python3
"""Compare local neo-riscv-fault log lines with reference getapplicationlog.

The goal is to catch "local FAULT, reference HALT" immediately and preserve a
small, reproducible oracle seed before state-root replay reaches the mismatch.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Iterable


FAULT_RE = re.compile(
    r"\[neo-riscv-fault\]\s+"
    r"block=(?P<block>\S+)\s+"
    r"trigger=(?P<trigger>\S+)\s+"
    r"container=(?P<container>\S+)\s+"
    r"context=(?P<context>\S+)\s+"
    r"ip=(?P<ip>\S+)\s+"
    r"message=(?P<message>.*)$"
)


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


def parse_fault_line(line: str) -> dict[str, str] | None:
    match = FAULT_RE.search(line.strip())
    return match.groupdict() if match else None


def select_execution(application_log: dict[str, Any] | None, trigger: str) -> dict[str, Any] | None:
    executions = (application_log or {}).get("executions") or []
    if not executions:
        return None
    for execution in executions:
        if str(execution.get("trigger")) == trigger:
            return execution
    return executions[0]


def classify(local: dict[str, str], reference_execution: dict[str, Any] | None, rpc_error: str | None) -> dict[str, Any]:
    record: dict[str, Any] = {
        "block": local["block"],
        "trigger": local["trigger"],
        "container": local["container"],
        "context": local["context"],
        "ip": local["ip"],
        "local_vmstate": "FAULT",
        "local_message": local["message"],
    }
    if rpc_error is not None:
        record.update(
            {
                "status": "reference_rpc_error",
                "reference_error": rpc_error,
                "reference_vmstate": None,
                "reference_exception": None,
            }
        )
        return record
    if reference_execution is None:
        record.update(
            {
                "status": "reference_log_missing",
                "reference_vmstate": None,
                "reference_exception": None,
            }
        )
        return record

    reference_vmstate = str(reference_execution.get("vmstate") or "")
    reference_exception = reference_execution.get("exception")
    if "HALT" in reference_vmstate and "FAULT" not in reference_vmstate:
        status = "local_fault_reference_halt"
    elif "FAULT" in reference_vmstate:
        status = "both_fault"
    else:
        status = "reference_other"

    record.update(
        {
            "status": status,
            "reference_vmstate": reference_vmstate,
            "reference_exception": reference_exception,
            "reference_stack": reference_execution.get("stack"),
            "reference_notifications": reference_execution.get("notifications"),
        }
    )
    return record


def extract_fault_records(lines: Iterable[str], reference_rpc: str, timeout: float = 20.0) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    for line in lines:
        parsed = parse_fault_line(line)
        if parsed is None:
            continue
        key = (parsed["block"], parsed["container"], parsed["ip"])
        if key in seen:
            continue
        seen.add(key)

        rpc_error = None
        execution = None
        try:
            application_log = rpc_call(reference_rpc, "getapplicationlog", [parsed["container"]], timeout=timeout)
            execution = select_execution(application_log, parsed["trigger"])
        except Exception as exc:  # preserve bad RPC responses as data, not a hard stop
            rpc_error = str(exc)
        records.append(classify(parsed, execution, rpc_error))
    return records


def write_outputs(records: list[dict[str, Any]], output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    jsonl_path = output_dir / "fault-oracle.jsonl"
    with jsonl_path.open("w", encoding="utf-8") as f:
        for record in records:
            f.write(json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n")

    seeds = [
        {
            "block": record["block"],
            "container": record["container"],
            "trigger": record["trigger"],
            "context": record["context"],
            "ip": record["ip"],
            "expected_reference_vmstate": record["reference_vmstate"],
            "local_message": record["local_message"],
        }
        for record in records
        if record.get("status") == "local_fault_reference_halt"
    ]
    (output_dir / "regression-seeds.json").write_text(
        json.dumps({"seeds": seeds}, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--log-file",
        default="mainnet-validation/logs/neo-cli-launchd.err",
        help="neo-cli stderr containing [neo-riscv-fault] lines",
    )
    parser.add_argument("--reference-rpc", default="http://seed1.neo.org:10332")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument(
        "--output-dir",
        default="tests/corpus/mainnet-fault-oracle",
        help="directory for fault-oracle.jsonl and regression-seeds.json",
    )
    parser.add_argument("--fail-on-reference-halt", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    log_path = Path(args.log_file)
    if not log_path.exists():
        print(f"fault log not found: {log_path}", file=sys.stderr)
        return 2

    records = extract_fault_records(log_path.read_text(encoding="utf-8", errors="replace").splitlines(), args.reference_rpc, args.timeout)
    write_outputs(records, Path(args.output_dir))

    halt_mismatches = [record for record in records if record.get("status") == "local_fault_reference_halt"]
    print(
        f"fault_oracle records={len(records)} local_fault_reference_halt={len(halt_mismatches)} output={args.output_dir}"
    )
    for record in halt_mismatches[:20]:
        print(
            "HALT_ORACLE "
            f"block={record['block']} tx={record['container']} context={record['context']} "
            f"ip={record['ip']} message={record['local_message']}"
        )

    return 1 if args.fail_on_reference_halt and halt_mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
