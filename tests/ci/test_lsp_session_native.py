"""Native timing must join request identity and preserve unmeasured intervals."""

import json
import tempfile
import unittest
from pathlib import Path

from scripts.lsp_tools.native import intervals, native_requests, recovery_wire_samples


class NativeTimingTests(unittest.TestCase):
    def test_wire_join_uses_server_identity_and_rejects_incomplete_or_reversed_intervals(self):
        report = {
            "server_pid": 42,
            "checkpoints": [{}] * 5 + [{"recovery_requests": {"rename": 7, "completion": 8}}],
        }
        events = [
            {"pid": 42, "id": 7, "event": "response", "received_ns": 3_000_000},
            {"pid": 99, "id": 7, "event": "response", "received_ns": 90_000_000},
            {"pid": 42, "id": 7, "event": "send", "started_ns": 1_000_000},
            {"pid": 42, "id": 8, "event": "send", "started_ns": 4_000_000},
            {"pid": 42, "id": 8, "event": "response", "received_ns": 4_500_000},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text("\n".join(map(json.dumps, events)))
            self.assertEqual(
                dict(recovery_wire_samples(report, path)), {"rename": [2.0], "completion": [0.5]}
            )
            path.write_text("\n".join(map(json.dumps, events[:-1])))
            with self.assertRaises(KeyError):
                recovery_wire_samples(report, path)
            events[-1]["received_ns"] = 3_000_000
            path.write_text("\n".join(map(json.dumps, events)))
            with self.assertRaises(ValueError):
                recovery_wire_samples(report, path)

    def test_serial_join_and_last_ready_time_survive_interleaved_file_writes(self):
        events = [
            (0.001, {"phase": "ingress", "id": "7"}),
            (0.002, {"phase": "queued", "id": "7", "serial": 42}),
            (0.003, {"phase": "query_dispatch", "serial": 42}),
            (0.014, {"phase": "query_start", "serial": 42}),
            (0.015, {"phase": "query_end", "serial": 42}),
            (0.026, {"phase": "query_observed", "serial": 42}),
            (0.027, {"phase": "output_ready", "id": "7"}),
            (0.028, {"phase": "output_ready", "id": "7"}),
            (0.030, {"phase": "handoff", "id": "7"}),
            (0.004, {"phase": "query_start", "serial": 99}),
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text(
                "\n".join(
                    json.dumps({"timestamp": f"{at}s", "fields": fields})
                    for at, fields in reversed(events)
                )
            )
            result = intervals(native_requests(path)["7"], 40)
        self.assertAlmostEqual(result["worker_wake_ms"], 11)
        self.assertAlmostEqual(result["result_wake_ms"], 11)
        self.assertAlmostEqual(result["execution_ms"], 1)
        self.assertAlmostEqual(result["server_ms"], 29)
        self.assertAlmostEqual(result["outside_server_ms"], 11)
        self.assertAlmostEqual(result["ready_handoff_ms"], 2)

    def test_cancelled_query_needs_no_worker_and_residual_stays_signed(self):
        result = intervals({"ingress": 0, "output_ready": 1, "handoff": 3}, 2)
        self.assertEqual(result["outside_server_ms"], -1)
        self.assertNotIn("execution_ms", result)
        with self.assertRaises(KeyError):
            intervals({"ingress": 0, "output_ready": 1}, 2)
