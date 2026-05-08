import importlib.util
import json
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "extract-fault-oracle-corpus.py"


def load_module():
    spec = importlib.util.spec_from_file_location("extract_fault_oracle_corpus", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FaultOracleCorpusTests(unittest.TestCase):
    def test_parses_fault_line_and_flags_reference_halt(self):
        module = load_module()
        line = (
            "[neo-riscv-fault] block=2219045 trigger=Application "
            "container=0x9020bf46edb30f5925f75c950d530f25a788fa3fa870386bf8f6db82271f9fe7 "
            "context=GenesMixer:_initialize:0x0daa5feb9d72602b0cb46479b65e85021419290e "
            "ip=29 message=bitwise op expects matching integer, boolean, or byte string types; ip=221"
        )

        with mock.patch.object(
            module,
            "rpc_call",
            return_value={
                "executions": [
                    {
                        "trigger": "Application",
                        "vmstate": "HALT",
                        "exception": None,
                        "stack": [{"type": "Boolean", "value": True}],
                    }
                ]
            },
        ):
            records = module.extract_fault_records([line], "http://seed1.neo.org:10332")

        self.assertEqual(1, len(records))
        record = records[0]
        self.assertEqual("2219045", record["block"])
        self.assertEqual("0x9020bf46edb30f5925f75c950d530f25a788fa3fa870386bf8f6db82271f9fe7", record["container"])
        self.assertEqual("HALT", record["reference_vmstate"])
        self.assertEqual("local_fault_reference_halt", record["status"])

    def test_writes_jsonl_and_regression_seed(self):
        module = load_module()
        record = {
            "status": "local_fault_reference_halt",
            "block": "7",
            "container": "0xabc",
            "trigger": "Application",
            "context": "Contract:method:0x123",
            "ip": "9",
            "local_message": "boom",
            "reference_vmstate": "HALT",
            "reference_exception": None,
        }

        out_dir = Path(self._testMethodName)
        try:
            module.write_outputs([record], out_dir)
            jsonl = out_dir / "fault-oracle.jsonl"
            seeds = out_dir / "regression-seeds.json"
            self.assertTrue(jsonl.exists())
            self.assertTrue(seeds.exists())
            self.assertEqual(record, json.loads(jsonl.read_text().strip()))
            seed_doc = json.loads(seeds.read_text())
            self.assertEqual(["0xabc"], [seed["container"] for seed in seed_doc["seeds"]])
        finally:
            for path in sorted(out_dir.glob("*"), reverse=True):
                path.unlink()
            if out_dir.exists():
                out_dir.rmdir()


if __name__ == "__main__":
    unittest.main()
