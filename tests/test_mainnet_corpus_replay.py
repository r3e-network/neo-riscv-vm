import importlib.util
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "replay-mainnet-oracle-corpus.py"


def load_module():
    spec = importlib.util.spec_from_file_location("replay_mainnet_oracle_corpus", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class MainnetCorpusReplayTests(unittest.TestCase):
    def test_replays_reference_applicationlog_against_local_rpc(self):
        module = load_module()
        records = [
            {
                "block": 10,
                "tx": "0xaaa",
                "execution_index": 0,
                "trigger": "Application",
                "vmstate": "HALT",
                "exception": None,
                "event_names": ["Transfer"],
                "notification_count": 1,
                "stack": [{"type": "Boolean", "value": True}],
            }
        ]

        with mock.patch.object(
            module,
            "rpc_call",
            return_value={
                "executions": [
                    {
                        "trigger": "Application",
                        "vmstate": "HALT",
                        "exception": None,
                        "notifications": [{"eventname": "Transfer"}],
                        "stack": [{"type": "Boolean", "value": True}],
                    }
                ]
            },
        ):
            result = module.replay_records(records, "http://127.0.0.1:10332")

        self.assertEqual(1, result.checked)
        self.assertEqual([], result.mismatches)
        self.assertEqual([], result.missing)

    def test_reports_vmstate_mismatch(self):
        module = load_module()
        records = [
            {
                "block": 10,
                "tx": "0xaaa",
                "execution_index": 0,
                "trigger": "Application",
                "vmstate": "HALT",
                "exception": None,
                "event_names": [],
                "notification_count": 0,
                "stack": [],
            }
        ]

        with mock.patch.object(
            module,
            "rpc_call",
            return_value={
                "executions": [
                    {
                        "trigger": "Application",
                        "vmstate": "FAULT",
                        "exception": "boom",
                        "notifications": [],
                        "stack": [],
                    }
                ]
            },
        ):
            result = module.replay_records(records, "http://127.0.0.1:10332")

        self.assertEqual(1, result.checked)
        self.assertGreaterEqual(len(result.mismatches), 1)
        self.assertIn("vmstate", [mismatch["field"] for mismatch in result.mismatches])


if __name__ == "__main__":
    unittest.main()
