#!/usr/bin/env python3
"""Compare Neo StateService roots between local and reference RPC endpoints.

The validator runs JSON-RPC batch requests against both endpoints and fails on
the first mismatch. It is intended for mainnet replay runs where the local node
is syncing with the RISC-V adapter and StateService enabled.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


_NO_PROXY_OPENER = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compare getstateroot results from local and reference RPCs."
    )
    parser.add_argument("--local-rpc", default="http://127.0.0.1:10332")
    parser.add_argument("--reference-rpc", default="http://seed1.neo.org:10332")
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--end", type=int)
    parser.add_argument("--lag", type=int, default=2)
    parser.add_argument("--batch-size", type=int, default=200)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--retries", type=int, default=4)
    parser.add_argument("--retry-sleep", type=float, default=2.0)
    parser.add_argument("--report-interval", type=int, default=5000)
    parser.add_argument("--log-file")
    parser.add_argument("--checkpoint-file")
    parser.add_argument("--follow", action="store_true")
    parser.add_argument("--poll-interval", type=float, default=10.0)
    return parser.parse_args()


def urlopen_without_system_proxy(request: urllib.request.Request, *, timeout: float):
    global _NO_PROXY_OPENER
    if _NO_PROXY_OPENER is None:
        _NO_PROXY_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    return _NO_PROXY_OPENER.open(request, timeout=timeout)


def rpc(
    url: str,
    payload: object,
    *,
    timeout: float,
    retries: int,
    retry_sleep: float,
) -> object:
    body = json.dumps(payload, separators=(",", ":")).encode()
    last_error: BaseException | None = None
    for attempt in range(1, retries + 1):
        try:
            request = urllib.request.Request(
                url,
                data=body,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urlopen_without_system_proxy(request, timeout=timeout) as response:
                return json.loads(response.read())
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
            last_error = exc
            if attempt == retries:
                break
            sleep_for = min(retry_sleep * attempt, 10.0)
            print(
                f"WARN rpc retry {attempt}/{retries} url={url} "
                f"error={exc}; sleep={sleep_for:.1f}s",
                flush=True,
            )
            time.sleep(sleep_for)
    raise RuntimeError(f"RPC failed for {url}: {last_error}")


def get_block_count(args: argparse.Namespace) -> int:
    response = rpc(
        args.local_rpc,
        {"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1},
        timeout=args.timeout,
        retries=args.retries,
        retry_sleep=args.retry_sleep,
    )
    if not isinstance(response, dict) or "result" not in response:
        raise RuntimeError(f"Invalid getblockcount response: {response!r}")
    return int(response["result"])


def roots_by_height(response: object) -> dict[int, str]:
    if isinstance(response, dict):
        responses = [response]
    elif isinstance(response, list):
        responses = response
    else:
        raise RuntimeError(f"Invalid batch response: {response!r}")

    roots: dict[int, str] = {}
    for item in responses:
        if not isinstance(item, dict):
            continue
        idx = item.get("id")
        if not isinstance(idx, int):
            continue
        result = item.get("result")
        if isinstance(result, dict) and isinstance(result.get("roothash"), str):
            roots[idx] = result["roothash"]
        else:
            roots[idx] = "ERROR:" + json.dumps(item, separators=(",", ":"))[:200]
    return roots


def state_root_batch(start: int, end: int) -> list[dict[str, object]]:
    return [
        {"jsonrpc": "2.0", "method": "getstateroot", "params": [idx], "id": idx}
        for idx in range(start, end + 1)
    ]


def state_roots_with_batch_fallback(
    args: argparse.Namespace,
    url: str,
    payload: list[dict[str, object]],
    *,
    depth: int = 0,
) -> dict[int, str]:
    request_payload: object = payload[0] if len(payload) == 1 else payload
    try:
        return roots_by_height(
            rpc(
                url,
                request_payload,
                timeout=args.timeout,
                retries=args.retries,
                retry_sleep=args.retry_sleep,
            )
        )
    except RuntimeError as exc:
        if len(payload) == 1:
            raise

        if depth == 0:
            first = payload[0].get("id")
            last = payload[-1].get("id")
            print(
                f"WARN stateroot batch failed url={url} "
                f"range={first}..{last} size={len(payload)}; "
                f"falling back to single requests: {exc}",
                flush=True,
            )

        roots: dict[int, str] = {}
        for request in payload:
            roots.update(
                state_roots_with_batch_fallback(
                    args,
                    url,
                    [request],
                    depth=depth + 1,
                )
            )
        return roots


def open_log(path: str | None):
    if path is None:
        return None
    log_path = Path(path)
    new_file = not log_path.exists() or log_path.stat().st_size == 0
    handle = log_path.open("a", encoding="utf-8")
    if new_file:
        handle.write("height\tlocal\treference\tstatus\n")
        handle.flush()
    return handle


def write_checkpoint(path: str | None, height: int) -> None:
    if path is None:
        return
    checkpoint_path = Path(path)
    checkpoint_path.write_text(f"{height}\n", encoding="utf-8")


def is_unknown_state_root(root: str) -> bool:
    return root.startswith("ERROR:") and "Unknown state root" in root


def resume_start_from_checkpoint(start: int, path: str | None) -> int:
    if path is None:
        return start

    checkpoint_path = Path(path)
    if not checkpoint_path.exists():
        return start

    text = checkpoint_path.read_text(encoding="utf-8").strip()
    if not text:
        return start

    try:
        checkpoint = int(text)
    except ValueError as exc:
        raise RuntimeError(f"Invalid checkpoint file {path}: {text!r}") from exc

    resumed = checkpoint + 1
    if resumed > start:
        print(
            f"Resuming from checkpoint {checkpoint}; start={resumed}",
            flush=True,
        )
        return resumed
    return start


def compare_range(
    args: argparse.Namespace,
    start: int,
    end: int,
    *,
    log_handle,
) -> tuple[int, int, int]:
    checked = 0
    next_report = start + args.report_interval

    for lo in range(start, end + 1, args.batch_size):
        hi = min(lo + args.batch_size - 1, end)
        payload = state_root_batch(lo, hi)
        local_roots = state_roots_with_batch_fallback(
            args,
            args.local_rpc,
            payload,
        )
        reference_roots = state_roots_with_batch_fallback(
            args,
            args.reference_rpc,
            payload,
        )

        for height in range(lo, hi + 1):
            local_root = local_roots.get(height, "MISSING_LOCAL")
            reference_root = reference_roots.get(height, "MISSING_REFERENCE")
            ok = (
                local_root == reference_root
                and isinstance(local_root, str)
                and local_root.startswith("0x")
            )
            if log_handle is not None:
                status = "OK" if ok else "MISMATCH"
                if ok:
                    log_handle.write(
                        f"{height}\t{local_root}\t{reference_root}\t{status}\n"
                    )
            if ok:
                checked += 1
                continue

            if (
                args.follow
                and isinstance(local_root, str)
                and isinstance(reference_root, str)
                and reference_root.startswith("0x")
                and is_unknown_state_root(local_root)
            ):
                if log_handle is not None:
                    log_handle.flush()
                if height > lo:
                    write_checkpoint(args.checkpoint_file, height - 1)
                print(
                    f"WAIT local state root not available height={height}; "
                    f"checkpoint={height - 1 if height > lo else start - 1}; "
                    f"retrying in follow mode",
                    flush=True,
                )
                return checked, -1, height

            if log_handle is not None:
                log_handle.write(
                    f"{height}\t{local_root}\t{reference_root}\tMISMATCH\n"
                )
                log_handle.flush()
            print(
                f"MISMATCH height={height} "
                f"local={local_root} reference={reference_root}",
                flush=True,
            )
            return checked, height, -1

        if log_handle is not None:
            log_handle.flush()
        write_checkpoint(args.checkpoint_file, hi)
        if hi >= next_report or hi == end:
            print(f"OK through {hi}; checked={checked}", flush=True)
            while next_report <= hi:
                next_report += args.report_interval

    return checked, -1, -1


def main() -> int:
    args = parse_args()
    if args.batch_size < 1:
        raise SystemExit("--batch-size must be positive")
    if args.lag < 0:
        raise SystemExit("--lag must be non-negative")

    start = resume_start_from_checkpoint(args.start, args.checkpoint_file)
    total_checked = 0
    log_handle = open_log(args.log_file)
    try:
        while True:
            try:
                end = args.end
                if end is None:
                    block_count = get_block_count(args)
                    end = block_count - 1 - args.lag
                    print(
                        f"Local blockcount={block_count}; comparing {start}..{end}",
                        flush=True,
                    )

                if end < start:
                    if not args.follow:
                        print(f"PASS no new state roots to compare from {start}", flush=True)
                        return 0
                    time.sleep(args.poll_interval)
                    continue

                checked, mismatch_height, pending_height = compare_range(args, start, end, log_handle=log_handle)
                total_checked += checked
                if mismatch_height >= 0:
                    return 2
                if pending_height >= 0:
                    start = pending_height
                    if not args.follow:
                        return 2
                    time.sleep(args.poll_interval)
                    continue

                print(
                    f"PASS compared every state root {start}..{end}; "
                    f"checked={checked}; total_checked={total_checked}; mismatches=0",
                    flush=True,
                )
                start = end + 1
                if not args.follow:
                    return 0
                time.sleep(args.poll_interval)
            except RuntimeError as exc:
                if not args.follow:
                    raise
                print(
                    f"WARN transient comparison failure: {exc}; "
                    f"retrying in {args.poll_interval:.1f}s",
                    flush=True,
                )
                time.sleep(args.poll_interval)
    finally:
        if log_handle is not None:
            log_handle.close()


if __name__ == "__main__":
    sys.exit(main())
