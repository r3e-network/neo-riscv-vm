#!/usr/bin/env python3
import argparse
import importlib.util
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "compare-stateroot-rpc-batch.py"


def load_module():
    spec = importlib.util.spec_from_file_location("compare_stateroot_rpc_batch", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class TransientFailureTests(unittest.TestCase):
    def test_follow_mode_retries_transient_rpc_failure_without_exiting(self):
        module = load_module()
        args = argparse.Namespace(
            local_rpc="http://127.0.0.1:10332",
            reference_rpc="http://seed1.neo.org:10332",
            start=0,
            end=None,
            lag=2,
            batch_size=512,
            timeout=30.0,
            retries=4,
            retry_sleep=2.0,
            report_interval=5120,
            log_file=None,
            checkpoint_file=None,
            follow=True,
            poll_interval=0.01,
        )

        class SleptAfterFailure(Exception):
            pass

        with (
            mock.patch.object(module, "parse_args", return_value=args),
            mock.patch.object(module, "get_block_count", side_effect=RuntimeError("RPC failed")),
            mock.patch.object(module.time, "sleep", side_effect=SleptAfterFailure),
        ):
            with self.assertRaises(SleptAfterFailure):
                module.main()


if __name__ == "__main__":
    unittest.main()
