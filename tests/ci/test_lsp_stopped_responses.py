"""Distinguish cancellation delivery from a success that won the cancellation race."""

import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

scripts = Path(__file__).resolve().parents[2] / "scripts"
sys.path.insert(0, str(scripts))
spec = importlib.util.spec_from_file_location("stopped", scripts / "measure-lsp-stopped-responses.py")
stopped = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stopped)


class StoppedResponsesTests(unittest.TestCase):
    def test_outcomes_remain_separate_and_wrong_driver_or_error_fails_closed(self):
        report = {"server_pid": 42, "provenance": {"harness_dirty": False},
                  "driver": {"native_trace": False, "server_environment": {}, "edit_interval_ms": 100,
                             "editing_cpu_accounting": True, "settled_idle_seconds": 3,
                             "settled_idle_location": "startup"}, "checkpoints": [{}] * 5}
        events = []
        for index, (code, rename_ms) in enumerate(((-32800, 20), (None, 1))):
            requests = {"rename": index * 2, "completion": index * 2 + 1}
            report["checkpoints"].append({"recovery_requests": requests,
                                          "recovery_outcomes": {"rename_error_code": code}})
            for name, request_id in requests.items():
                events.extend([{"pid": 42, "id": request_id, "event": "send", "started_ns": 0},
                               {"pid": 42, "id": request_id, "event": "response",
                                "received_ns": (rename_ms if name == "rename" else .5) * 1e6}])
        with tempfile.TemporaryDirectory() as temporary:
            trace = Path(temporary) / "trace.jsonl"
            trace.write_text("\n".join(map(json.dumps, events)))
            result = stopped.wire_workload(report, trace)
            self.assertEqual(result["rename_outcomes"]["cancelled"]["over_10_ms"], 1)
            self.assertEqual(result["rename_outcomes"]["success"]["over_10_ms"], 0)
            for corruption in ("dirty", "tracing", "environment", "pacing", "accounting", "idle", "error", "missing"):
                invalid = copy.deepcopy(report)
                if corruption == "dirty":
                    invalid["provenance"]["harness_dirty"] = True
                elif corruption == "error":
                    invalid["checkpoints"][-1]["recovery_outcomes"]["rename_error_code"] = -32803
                elif corruption == "missing":
                    del invalid["checkpoints"][-1]["recovery_outcomes"]
                else:
                    key, value = {"tracing": ("native_trace", True), "environment": ("server_environment", {"PROBE": "1"}),
                                  "pacing": ("edit_interval_ms", 5), "accounting": ("editing_cpu_accounting", False),
                                  "idle": ("settled_idle_location", "after_cycles")}[corruption]
                    invalid["driver"][key] = value
                with self.subTest(corruption=corruption), self.assertRaises((KeyError, ValueError)):
                    stopped.wire_workload(invalid, trace)
