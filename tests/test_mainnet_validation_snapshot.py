import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "mainnet-validation-snapshot.py"


def load_snapshot_module():
    spec = importlib.util.spec_from_file_location("mainnet_validation_snapshot", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class MainnetValidationSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.module = load_snapshot_module()

    def make_snapshot(self, root, height, node_height):
        path = root / f"height-{height:012d}"
        path.mkdir(parents=True)
        for name in self.module.DATA_DIRS:
            db_dir = path / name
            db_dir.mkdir()
            (db_dir / "marker.txt").write_text(f"{height}:{name}", encoding="utf-8")
        (path / "manifest.json").write_text(
            json.dumps({"height": height, "node_height": node_height}),
            encoding="utf-8",
        )
        return path

    def test_select_latest_snapshot_before_height_uses_node_height(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.make_snapshot(root, height=1000, node_height=1100)
            self.make_snapshot(root, height=2000, node_height=2500)
            self.make_snapshot(root, height=3000, node_height=3200)
            (root / "height-999999999999.tmp").mkdir()
            (root / "not-a-snapshot").mkdir()

            selected = self.module.select_snapshot(root, before_height=2600)
            self.assertEqual(selected.height, 2000)
            self.assertEqual(selected.node_height, 2500)

            selected = self.module.select_snapshot(root, before_height=2400)
            self.assertEqual(selected.height, 1000)
            self.assertEqual(selected.node_height, 1100)

    def test_save_snapshot_skips_when_latest_checkpoint_is_too_recent(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            validation_dir = tmp_path / "validation"
            snapshot_root = tmp_path / "snapshots"
            validation_dir.mkdir()
            snapshot_root.mkdir()
            for name in self.module.DATA_DIRS:
                db_dir = validation_dir / name
                db_dir.mkdir()
                (db_dir / "current.txt").write_text(name, encoding="utf-8")
            self.make_snapshot(snapshot_root, height=1000, node_height=1010)

            result = self.module.save_snapshot(
                validation_dir=validation_dir,
                snapshot_root=snapshot_root,
                height=1050,
                node_height=1060,
                min_interval=100,
                keep=3,
                copy_tree=self.module.copy_tree_shutil,
            )

            self.assertFalse(result.saved)
            self.assertEqual(result.reason, "too-recent")
            self.assertEqual(len(self.module.list_snapshots(snapshot_root)), 1)

    def test_apfs_clone_fallback_removes_partial_destination(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            dst = tmp_path / "dst"
            src.mkdir()
            (src / "marker.txt").write_text("copied", encoding="utf-8")

            original_platform = self.module.sys.platform
            original_run = self.module.subprocess.run

            class FailedCp:
                returncode = 1

            def fake_run(*args, **kwargs):
                dst.mkdir()
                (dst / "partial.txt").write_text("partial", encoding="utf-8")
                return FailedCp()

            try:
                self.module.sys.platform = "darwin"
                self.module.subprocess.run = fake_run
                self.module.copy_tree_apfs_clone(src, dst)
            finally:
                self.module.subprocess.run = original_run
                self.module.sys.platform = original_platform

            self.assertEqual((dst / "marker.txt").read_text(encoding="utf-8"), "copied")
            self.assertFalse((dst / "partial.txt").exists())

    def test_save_snapshot_retries_transient_copy_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            validation_dir = tmp_path / "validation"
            snapshot_root = tmp_path / "snapshots"
            validation_dir.mkdir()
            snapshot_root.mkdir()
            for name in self.module.DATA_DIRS:
                db_dir = validation_dir / name
                db_dir.mkdir()
                (db_dir / "current.txt").write_text(name, encoding="utf-8")

            calls = []

            def flaky_copy(src, dst):
                if not calls:
                    calls.append((src, dst))
                    raise FileNotFoundError("source file disappeared during live DB copy")
                self.module.copy_tree_shutil(src, dst)

            result = self.module.save_snapshot(
                validation_dir=validation_dir,
                snapshot_root=snapshot_root,
                height=2000,
                node_height=2010,
                min_interval=100,
                keep=3,
                copy_attempts=2,
                retry_delay=0,
                copy_tree=flaky_copy,
            )

            self.assertTrue(result.saved)
            self.assertEqual(result.reason, "saved")
            self.assertEqual(len(calls), 1)
            self.assertEqual(len(self.module.list_snapshots(snapshot_root)), 1)

    def test_restore_snapshot_replaces_databases_and_seeds_checkpoint(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            validation_dir = tmp_path / "validation"
            snapshot_root = tmp_path / "snapshots"
            archive_root = validation_dir / "archive"
            logs_dir = validation_dir / "logs"
            validation_dir.mkdir()
            snapshot_root.mkdir()
            logs_dir.mkdir()
            for name in self.module.DATA_DIRS:
                db_dir = validation_dir / name
                db_dir.mkdir()
                (db_dir / "current.txt").write_text("old", encoding="utf-8")
            (logs_dir / "stateroot-continuous.checkpoint").write_text("77\n", encoding="utf-8")
            (logs_dir / "stateroot-continuous.tsv").write_text("old log\n", encoding="utf-8")
            self.make_snapshot(snapshot_root, height=12345, node_height=12400)

            result = self.module.restore_snapshot(
                validation_dir=validation_dir,
                snapshot_root=snapshot_root,
                archive_root=archive_root,
                logs_dir=logs_dir,
                before_height=13000,
                reason="mismatch-13000",
                copy_tree=self.module.copy_tree_shutil,
            )

            self.assertEqual(result.snapshot.height, 12345)
            for name in self.module.DATA_DIRS:
                self.assertFalse((validation_dir / name / "current.txt").exists())
                self.assertEqual(
                    (validation_dir / name / "marker.txt").read_text(encoding="utf-8"),
                    f"12345:{name}",
                )
            self.assertEqual((logs_dir / "stateroot-continuous.checkpoint").read_text(encoding="utf-8"), "12345\n")
            self.assertEqual(
                (logs_dir / "stateroot-continuous.tsv").read_text(encoding="utf-8"),
                "height\tlocal\treference\tstatus\n",
            )
            self.assertTrue((result.archive_dir / "logs" / "stateroot-continuous.tsv").exists())
            self.assertTrue((result.archive_dir / self.module.DATA_DIRS[0] / "current.txt").exists())


if __name__ == "__main__":
    unittest.main()
