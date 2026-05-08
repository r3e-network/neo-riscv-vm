import importlib.util
import json
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "run-stateroot-segments.py"


def load_module():
    spec = importlib.util.spec_from_file_location("run_stateroot_segments", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class StaterootSegmentTests(unittest.TestCase):
    def test_loads_segments_and_builds_compare_commands(self):
        module = load_module()
        manifest = Path(self._testMethodName) / "segments.json"
        manifest.parent.mkdir(exist_ok=True)
        try:
            manifest.write_text(
                json.dumps(
                    {
                        "segments": [
                            {"name": "bitwise-regression", "start": 2219000, "end": 2219100},
                            {"name": "assert-heavy", "start": 44000, "end": 44600},
                        ]
                    }
                )
            )
            segments = module.load_segments(manifest)
            commands = [
                module.build_compare_command(
                    segment,
                    local_rpc="http://127.0.0.1:10332",
                    reference_rpc="http://seed1.neo.org:10332",
                    batch_size=128,
                )
                for segment in segments
            ]
        finally:
            if manifest.exists():
                manifest.unlink()
            if manifest.parent.exists():
                manifest.parent.rmdir()

        self.assertEqual(["bitwise-regression", "assert-heavy"], [segment["name"] for segment in segments])
        self.assertIn("--start", commands[0])
        self.assertIn("2219000", commands[0])
        self.assertIn("--end", commands[0])
        self.assertIn("2219100", commands[0])


if __name__ == "__main__":
    unittest.main()
