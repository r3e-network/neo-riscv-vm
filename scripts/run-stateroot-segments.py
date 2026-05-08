#!/usr/bin/env python3
"""Run state-root comparison over named high-risk block ranges."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[1]
COMPARE_SCRIPT = ROOT_DIR / "scripts" / "compare-stateroot-rpc-batch.py"


def load_segments(path: Path) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    segments = data.get("segments")
    if not isinstance(segments, list):
        raise ValueError(f"{path} must contain a 'segments' array")
    normalized: list[dict[str, Any]] = []
    for segment in segments:
        name = str(segment["name"])
        start = int(segment["start"])
        end = int(segment["end"])
        if end < start:
            raise ValueError(f"segment {name} has end < start")
        normalized.append({**segment, "name": name, "start": start, "end": end})
    return normalized


def build_compare_command(
    segment: dict[str, Any],
    *,
    local_rpc: str,
    reference_rpc: str,
    batch_size: int,
) -> list[str]:
    return [
        sys.executable,
        str(COMPARE_SCRIPT),
        "--local-rpc",
        local_rpc,
        "--reference-rpc",
        reference_rpc,
        "--start",
        str(segment["start"]),
        "--end",
        str(segment["end"]),
        "--batch-size",
        str(batch_size),
    ]


def run_segment(segment: dict[str, Any], command: list[str]) -> tuple[str, int]:
    print(f"segment_start name={segment['name']} range={segment['start']}..{segment['end']}")
    completed = subprocess.run(command, cwd=ROOT_DIR)
    print(f"segment_done name={segment['name']} status={completed.returncode}")
    return segment["name"], completed.returncode


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default="scripts/stateroot-segments.json")
    parser.add_argument("--local-rpc", default="http://127.0.0.1:10332")
    parser.add_argument("--reference-rpc", default="http://seed1.neo.org:10332")
    parser.add_argument("--batch-size", type=int, default=512)
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--name", action="append", help="run only matching segment name; may be repeated")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    segments = load_segments(Path(args.manifest))
    if args.name:
        wanted = set(args.name)
        segments = [segment for segment in segments if segment["name"] in wanted]
    if not segments:
        print("no segments selected", file=sys.stderr)
        return 64

    commands = [
        (
            segment,
            build_compare_command(
                segment,
                local_rpc=args.local_rpc,
                reference_rpc=args.reference_rpc,
                batch_size=args.batch_size,
            ),
        )
        for segment in segments
    ]

    if args.dry_run:
        for segment, command in commands:
            print(f"{segment['name']}: {' '.join(command)}")
        return 0

    failures: list[str] = []
    if args.jobs <= 1:
        for segment, command in commands:
            name, status = run_segment(segment, command)
            if status != 0:
                failures.append(name)
    else:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = [executor.submit(run_segment, segment, command) for segment, command in commands]
            for future in concurrent.futures.as_completed(futures):
                name, status = future.result()
                if status != 0:
                    failures.append(name)

    if failures:
        print(f"segment_failures count={len(failures)} names={','.join(sorted(failures))}", file=sys.stderr)
        return 1
    print(f"segment_pass count={len(commands)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
