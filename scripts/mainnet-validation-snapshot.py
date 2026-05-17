#!/usr/bin/env python3
"""Manage resumable DB snapshots for mainnet state-root validation.

The comparator checkpoint alone is not enough to resume after a root mismatch:
once the node has executed a bad block, later state can be contaminated. These
snapshots preserve both Neo DB directories plus the verified comparator height,
so a fixed build can restore the latest DB head before the mismatch and continue
without replaying from genesis.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable, NamedTuple


DATA_DIRS = ("Data_LevelDB_334F454E", "Data_MPT_334F454E")
STATEROOT_TSV_HEADER = "height\tlocal\treference\tstatus\n"
SNAPSHOT_RE = re.compile(r"^height-(\d+)$")


class SnapshotError(RuntimeError):
    pass


class Snapshot(NamedTuple):
    height: int
    node_height: int
    path: Path
    manifest: dict[str, object]


class SaveResult(NamedTuple):
    saved: bool
    reason: str
    snapshot_path: Path | None
    height: int
    node_height: int


class RestoreResult(NamedTuple):
    snapshot: Snapshot
    archive_dir: Path
    mpt_repair: str | None


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def default_validation_dir() -> Path:
    return repo_root() / "mainnet-validation"


def default_snapshot_root(validation_dir: Path) -> Path:
    return validation_dir / "resume-snapshots"


def snapshot_name(height: int) -> str:
    return f"height-{height:012d}"


def now_stamp() -> str:
    return time.strftime("%Y%m%d-%H%M%S")


def sanitize_reason(reason: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9._-]+", "-", reason.strip())
    return cleaned.strip("-") or "manual"


def load_snapshot(path: Path) -> Snapshot | None:
    match = SNAPSHOT_RE.match(path.name)
    if match is None or not path.is_dir():
        return None

    manifest_path = path / "manifest.json"
    if not manifest_path.exists():
        return None

    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        height = int(manifest.get("height", match.group(1)))
        node_height = int(manifest.get("node_height", height))
    except (OSError, TypeError, ValueError, json.JSONDecodeError):
        return None

    for name in DATA_DIRS:
        if not (path / name).is_dir():
            return None

    return Snapshot(height=height, node_height=node_height, path=path, manifest=manifest)


def list_snapshots(snapshot_root: Path) -> list[Snapshot]:
    if not snapshot_root.exists():
        return []
    snapshots = [snapshot for child in snapshot_root.iterdir() if (snapshot := load_snapshot(child)) is not None]
    return sorted(snapshots, key=lambda item: (item.node_height, item.height, str(item.path)))


def select_snapshot(
    snapshot_root: Path,
    *,
    before_height: int | None = None,
    at_or_before_height: int | None = None,
) -> Snapshot:
    snapshots = list_snapshots(snapshot_root)
    if before_height is not None:
        snapshots = [snapshot for snapshot in snapshots if snapshot.node_height < before_height]
    if at_or_before_height is not None:
        snapshots = [snapshot for snapshot in snapshots if snapshot.height <= at_or_before_height]

    if not snapshots:
        criteria = []
        if before_height is not None:
            criteria.append(f"node_height < {before_height}")
        if at_or_before_height is not None:
            criteria.append(f"height <= {at_or_before_height}")
        detail = " and ".join(criteria) if criteria else "any height"
        raise SnapshotError(f"No snapshot found for {detail} in {snapshot_root}")

    return max(snapshots, key=lambda item: (item.node_height, item.height, str(item.path)))


def copy_tree_shutil(src: Path, dst: Path) -> None:
    shutil.copytree(src, dst, symlinks=True)


def copy_tree_apfs_clone(src: Path, dst: Path) -> None:
    if dst.exists():
        raise SnapshotError(f"Destination already exists: {dst}")

    if sys.platform == "darwin":
        result = subprocess.run(
            ["cp", "-cR", str(src), str(dst)],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            return
        if dst.exists():
            shutil.rmtree(dst, ignore_errors=True)

    copy_tree_shutil(src, dst)


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def validate_source_dirs(validation_dir: Path) -> None:
    missing = [name for name in DATA_DIRS if not (validation_dir / name).is_dir()]
    if missing:
        raise SnapshotError(f"Missing DB directories under {validation_dir}: {', '.join(missing)}")


def prune_snapshots(snapshot_root: Path, keep: int) -> None:
    if keep <= 0:
        return
    snapshots = list_snapshots(snapshot_root)
    for snapshot in snapshots[:-keep]:
        shutil.rmtree(snapshot.path)


def save_snapshot(
    *,
    validation_dir: Path,
    snapshot_root: Path,
    height: int,
    node_height: int,
    min_interval: int,
    keep: int,
    copy_attempts: int = 3,
    retry_delay: float = 2.0,
    copy_tree: Callable[[Path, Path], None] = copy_tree_apfs_clone,
) -> SaveResult:
    if height < 0 or node_height < 0:
        raise SnapshotError("height and node_height must be non-negative")

    validate_source_dirs(validation_dir)
    snapshot_root.mkdir(parents=True, exist_ok=True)

    existing = list_snapshots(snapshot_root)
    if existing:
        latest_by_checkpoint = max(existing, key=lambda item: item.height)
        if height - latest_by_checkpoint.height < min_interval:
            return SaveResult(False, "too-recent", latest_by_checkpoint.path, height, node_height)

    final_path = snapshot_root / snapshot_name(height)
    if final_path.exists():
        return SaveResult(False, "exists", final_path, height, node_height)

    tmp_path = snapshot_root / f"{final_path.name}.tmp-{os.getpid()}"
    attempts = max(1, copy_attempts)
    for attempt in range(1, attempts + 1):
        if tmp_path.exists():
            shutil.rmtree(tmp_path, ignore_errors=True)
        tmp_path.mkdir(parents=True)

        try:
            for name in DATA_DIRS:
                copy_tree(validation_dir / name, tmp_path / name)

            manifest = {
                "schema_version": 1,
                "height": height,
                "node_height": node_height,
                "created_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
                "validation_dir": str(validation_dir),
                "data_dirs": list(DATA_DIRS),
                "note": "checkpoint height is verified; node_height is the DB head at snapshot time",
            }
            write_json(tmp_path / "manifest.json", manifest)
            tmp_path.rename(final_path)
            break
        except Exception:
            if tmp_path.exists():
                shutil.rmtree(tmp_path, ignore_errors=True)
            if attempt >= attempts:
                raise
            time.sleep(retry_delay)

    prune_snapshots(snapshot_root, keep)
    return SaveResult(True, "saved", final_path, height, node_height)


def unique_archive_dir(archive_root: Path, reason: str) -> Path:
    archive_root.mkdir(parents=True, exist_ok=True)
    base = archive_root / f"restore-{sanitize_reason(reason)}-{now_stamp()}"
    candidate = base
    suffix = 1
    while candidate.exists():
        suffix += 1
        candidate = Path(f"{base}-{suffix}")
    candidate.mkdir(parents=True)
    return candidate


def seed_resume_logs(logs_dir: Path, checkpoint: int) -> None:
    logs_dir.mkdir(parents=True, exist_ok=True)
    (logs_dir / "stateroot-continuous.checkpoint").write_text(f"{checkpoint}\n", encoding="utf-8")
    (logs_dir / "stateroot-continuous.tsv").write_text(STATEROOT_TSV_HEADER, encoding="utf-8")


def repair_mpt_current_local_root_index(validation_dir: Path, index: int) -> str:
    probe = repo_root() / "tools" / "LevelDbProbe" / "LevelDbProbe.csproj"
    if not probe.exists():
        raise SnapshotError(f"LevelDbProbe project not found: {probe}")

    env = os.environ.copy()
    dyld_paths = ["/opt/homebrew/lib", str(validation_dir / "Plugins" / "LevelDBStore")]
    existing_dyld = env.get("DYLD_LIBRARY_PATH")
    if existing_dyld:
        dyld_paths.append(existing_dyld)
    env["DYLD_LIBRARY_PATH"] = ":".join(dyld_paths)

    height_result = subprocess.run(
        [
            "dotnet",
            "run",
            "--project",
            str(probe),
            "-c",
            "Release",
            "--",
            "ledger-height",
            str(validation_dir / "Data_LevelDB_334F454E"),
        ],
        cwd=repo_root(),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if height_result.returncode != 0:
        raise SnapshotError(
            "Failed to read restored Ledger.CurrentIndex: "
            f"{height_result.stderr.strip() or height_result.stdout.strip() or f'exit {height_result.returncode}'}"
        )

    match = re.search(r"LedgerCurrentIndex=(\d+)", height_result.stdout)
    if match is None:
        raise SnapshotError(f"Could not parse restored Ledger.CurrentIndex from: {height_result.stdout.strip()}")

    ledger_index = int(match.group(1))
    result = subprocess.run(
        [
            "dotnet",
            "run",
            "--project",
            str(probe),
            "-c",
            "Release",
            "--",
            "set-local-root-index",
            str(validation_dir / "Data_MPT_334F454E"),
            str(ledger_index),
        ],
        cwd=repo_root(),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise SnapshotError(
            "Failed to repair MPT current local root index: "
            f"{result.stderr.strip() or result.stdout.strip() or f'exit {result.returncode}'}"
        )
    return f"LedgerCurrentIndex={ledger_index}; {result.stdout.strip()}; manifest_node_height={index}"


def restore_snapshot(
    *,
    validation_dir: Path,
    snapshot_root: Path,
    archive_root: Path,
    logs_dir: Path,
    before_height: int | None,
    reason: str,
    copy_tree: Callable[[Path, Path], None] = copy_tree_apfs_clone,
    repair_mpt_index: Callable[[Path, int], str] | None = None,
) -> RestoreResult:
    snapshot = select_snapshot(snapshot_root, before_height=before_height)
    validation_dir.mkdir(parents=True, exist_ok=True)
    archive_dir = unique_archive_dir(archive_root, reason)

    for name in DATA_DIRS:
        current = validation_dir / name
        if current.exists():
            shutil.move(str(current), str(archive_dir / name))

    if logs_dir.exists():
        shutil.move(str(logs_dir), str(archive_dir / "logs"))

    for name in DATA_DIRS:
        copy_tree(snapshot.path / name, validation_dir / name)

    mpt_repair = repair_mpt_index(validation_dir, snapshot.node_height) if repair_mpt_index is not None else None
    seed_resume_logs(logs_dir, snapshot.height)
    write_json(
        archive_dir / "resume-restore-manifest.json",
        {
            "restored_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "reason": reason,
            "before_height": before_height,
            "snapshot_path": str(snapshot.path),
            "snapshot_height": snapshot.height,
            "snapshot_node_height": snapshot.node_height,
            "mpt_repair": mpt_repair,
        },
    )
    return RestoreResult(snapshot=snapshot, archive_dir=archive_dir, mpt_repair=mpt_repair)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    save = subparsers.add_parser("save", help="Create a resumable DB snapshot")
    save.add_argument("--validation-dir", type=Path, default=default_validation_dir())
    save.add_argument("--snapshot-root", type=Path)
    save.add_argument("--height", type=int, required=True)
    save.add_argument("--node-height", type=int)
    save.add_argument("--min-interval", type=int, default=100000)
    save.add_argument("--keep", type=int, default=8)
    save.add_argument("--copy-attempts", type=int, default=3)
    save.add_argument("--retry-delay", type=float, default=2.0)
    save.add_argument("--no-clone", action="store_true", help="Use shutil copy instead of APFS clone")
    save.add_argument("--quiet", action="store_true")

    latest = subparsers.add_parser("latest", help="Print the selected snapshot")
    latest.add_argument("--snapshot-root", type=Path)
    latest.add_argument("--validation-dir", type=Path, default=default_validation_dir())
    latest.add_argument("--before-height", type=int)
    latest.add_argument("--at-or-before-height", type=int)
    latest.add_argument("--json", action="store_true")

    restore = subparsers.add_parser("restore", help="Restore the latest snapshot before a mismatch height")
    restore.add_argument("--validation-dir", type=Path, default=default_validation_dir())
    restore.add_argument("--snapshot-root", type=Path)
    restore.add_argument("--archive-root", type=Path)
    restore.add_argument("--logs-dir", type=Path)
    restore.add_argument("--before-height", type=int)
    restore.add_argument("--reason", default="manual")
    restore.add_argument("--no-clone", action="store_true", help="Use shutil copy instead of APFS clone")
    restore.add_argument("--no-repair-mpt-index", action="store_true", help="Do not reset MPT current local root index to snapshot node_height")

    return parser.parse_args(argv)


def snapshot_root_from_args(args: argparse.Namespace) -> Path:
    return args.snapshot_root or default_snapshot_root(args.validation_dir)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        if args.command == "save":
            snapshot_root = snapshot_root_from_args(args)
            node_height = args.node_height if args.node_height is not None else args.height
            copy_tree = copy_tree_shutil if args.no_clone else copy_tree_apfs_clone
            result = save_snapshot(
                validation_dir=args.validation_dir,
                snapshot_root=snapshot_root,
                height=args.height,
                node_height=node_height,
                min_interval=args.min_interval,
                keep=args.keep,
                copy_attempts=args.copy_attempts,
                retry_delay=args.retry_delay,
                copy_tree=copy_tree,
            )
            if not args.quiet:
                path = result.snapshot_path or snapshot_root / snapshot_name(args.height)
                print(
                    f"snapshot_{result.reason} height={result.height} "
                    f"node_height={result.node_height} path={path}"
                )
            return 0

        if args.command == "latest":
            snapshot = select_snapshot(
                snapshot_root_from_args(args),
                before_height=args.before_height,
                at_or_before_height=args.at_or_before_height,
            )
            if args.json:
                print(json.dumps(snapshot.manifest | {"path": str(snapshot.path)}, sort_keys=True))
            else:
                print(f"height={snapshot.height} node_height={snapshot.node_height} path={snapshot.path}")
            return 0

        if args.command == "restore":
            copy_tree = copy_tree_shutil if args.no_clone else copy_tree_apfs_clone
            result = restore_snapshot(
                validation_dir=args.validation_dir,
                snapshot_root=snapshot_root_from_args(args),
                archive_root=args.archive_root or args.validation_dir / "archive",
                logs_dir=args.logs_dir or args.validation_dir / "logs",
                before_height=args.before_height,
                reason=args.reason,
                copy_tree=copy_tree,
                repair_mpt_index=None if args.no_repair_mpt_index else repair_mpt_current_local_root_index,
            )
            print(
                f"restored_snapshot height={result.snapshot.height} "
                f"node_height={result.snapshot.node_height} "
                f"path={result.snapshot.path} archive={result.archive_dir}"
            )
            return 0
    except SnapshotError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    raise AssertionError(f"Unhandled command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
