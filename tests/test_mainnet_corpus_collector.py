import importlib.util
import json
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "collect-mainnet-oracle-corpus.py"


def load_module():
    spec = importlib.util.spec_from_file_location("collect_mainnet_oracle_corpus", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class MainnetCorpusCollectorTests(unittest.TestCase):
    def test_collects_application_logs_and_buckets_by_vmstate_exception_and_events(self):
        module = load_module()
        block = {"tx": [{"hash": "0xaaa"}, {"hash": "0xbbb"}]}
        logs = {
            "0xaaa": {
                "executions": [
                    {
                        "trigger": "Application",
                        "vmstate": "HALT",
                        "exception": None,
                        "notifications": [{"eventname": "Transfer"}],
                    }
                ]
            },
            "0xbbb": {
                "executions": [
                    {
                        "trigger": "Application",
                        "vmstate": "FAULT",
                        "exception": "ASSERT failed",
                        "notifications": [],
                    }
                ]
            },
        }

        def fake_rpc(_url, method, params, **_kwargs):
            if method == "getblock":
                return block
            if method == "getapplicationlog":
                return logs[params[0]]
            raise AssertionError(method)

        with mock.patch.object(module, "rpc_call", side_effect=fake_rpc):
            records = module.collect_range("http://seed1.neo.org:10332", 10, 10)

        self.assertEqual(2, len(records))
        self.assertEqual("vmstate=HALT;exception=none;events=Transfer", records[0]["bucket"])
        self.assertEqual("vmstate=FAULT;exception=ASSERT failed;events=none", records[1]["bucket"])

    def test_collects_raw_transaction_and_contract_nef_payloads(self):
        module = load_module()
        block = {"tx": [{"hash": "0xaaa"}]}
        app_log = {
            "executions": [
                {
                    "trigger": "Application",
                    "vmstate": "HALT",
                    "exception": None,
                    "notifications": [{"contract": "0xcontract", "eventname": "Transfer"}],
                }
            ]
        }
        raw_tx = {"hash": "0xaaa", "script": "DEADBEEF", "signers": [{"account": "0xsigner"}]}
        contract_state = {
            "hash": "0xcontract",
            "nef": {"checksum": 1234, "script": "AAECAw=="},
            "manifest": {"name": "Token"},
        }

        def fake_rpc(_url, method, params, **_kwargs):
            if method == "getblock":
                return block
            if method == "getapplicationlog":
                return app_log
            if method == "getrawtransaction":
                return raw_tx
            if method == "getcontractstate":
                return contract_state
            raise AssertionError(method)

        with mock.patch.object(module, "rpc_call", side_effect=fake_rpc):
            records = module.collect_range(
                "http://seed1.neo.org:10332",
                10,
                10,
                include_raw_transactions=True,
                include_contracts=True,
            )

        self.assertEqual(raw_tx, records[0]["raw_transaction"])
        self.assertEqual([contract_state], records[0]["contracts"])
        self.assertEqual(["0xcontract"], records[0]["contract_hashes"])

    def test_writes_corpus_jsonl(self):
        module = load_module()
        out_file = Path(self._testMethodName) / "corpus.jsonl"
        records = [{"block": 1, "tx": "0xaaa", "bucket": "vmstate=HALT;exception=none;events=Transfer"}]
        try:
            module.write_jsonl(records, out_file)
            self.assertEqual(records[0], json.loads(out_file.read_text().strip()))
        finally:
            if out_file.exists():
                out_file.unlink()
            if out_file.parent.exists():
                out_file.parent.rmdir()


if __name__ == "__main__":
    unittest.main()
