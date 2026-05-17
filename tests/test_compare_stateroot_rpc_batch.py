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
    def test_rpc_bypasses_system_proxy_settings(self):
        module = load_module()

        class FakeResponse:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def read(self):
                return b'{"jsonrpc":"2.0","id":1,"result":42}'

        class FakeOpener:
            def open(self, request, timeout):
                return FakeResponse()

        with (
            mock.patch.object(
                module.urllib.request,
                "urlopen",
                side_effect=AssertionError("default urlopen should not be used"),
            ),
            mock.patch.object(
                module.urllib.request,
                "build_opener",
                return_value=FakeOpener(),
            ) as build_opener,
        ):
            response = module.rpc(
                "http://seed1.neo.org:10332",
                {"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1},
                timeout=1.0,
                retries=1,
                retry_sleep=0.0,
            )

        self.assertEqual(response["result"], 42)
        build_opener.assert_called_once()

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

    def test_compare_range_splits_failed_reference_batch_to_single_requests(self):
        module = load_module()
        args = argparse.Namespace(
            local_rpc="http://127.0.0.1:10332",
            reference_rpc="http://seed1.neo.org:10332",
            batch_size=4,
            timeout=30.0,
            retries=1,
            retry_sleep=0.0,
            report_interval=100,
            checkpoint_file=None,
        )
        roots = {
            height: f"0x{height:064x}"
            for height in range(1602649, 1602653)
        }
        reference_single_requests = []

        def fake_rpc(url, payload, *, timeout, retries, retry_sleep):
            if (
                url == args.reference_rpc
                and isinstance(payload, list)
                and len(payload) > 1
            ):
                raise RuntimeError("HTTP Error 502: Bad Gateway")

            requests = payload if isinstance(payload, list) else [payload]
            if url == args.reference_rpc and not isinstance(payload, list):
                reference_single_requests.append(payload["id"])

            responses = [
                {
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": {
                        "version": 0,
                        "index": request["id"],
                        "roothash": roots[request["id"]],
                        "witnesses": [],
                    },
                }
                for request in requests
            ]
            return responses if isinstance(payload, list) else responses[0]

        with mock.patch.object(module, "rpc", side_effect=fake_rpc):
            checked, mismatch_height, pending_height = module.compare_range(
                args,
                1602649,
                1602652,
                log_handle=None,
            )

        self.assertEqual((checked, mismatch_height, pending_height), (4, -1, -1))
        self.assertEqual(
            reference_single_requests,
            [1602649, 1602650, 1602651, 1602652],
        )


if __name__ == "__main__":
    unittest.main()
