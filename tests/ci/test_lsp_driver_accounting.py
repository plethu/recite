"""Accounting experiments must differ only in the intended observer intervention."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

scripts = Path(__file__).resolve().parents[2] / "scripts"
sys.path.insert(0, str(scripts))
spec = importlib.util.spec_from_file_location("driver_accounting", scripts / "measure-lsp-driver-accounting.py")
accounting = importlib.util.module_from_spec(spec)
spec.loader.exec_module(accounting)


class DriverAccountingTests(unittest.TestCase):
    def test_one_intended_flag_difference_preserves_identity_and_wire_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for repetition in range(1, 4):
                for enabled in (False, True):
                    report = {
                        "status": "pass", "health": {"status": "pass"}, "cycles": 40,
                        "edits_per_cycle": 50, "churn": False, "seed": 7203,
                        "result_fingerprint_version": 2, "server_pid": repetition,
                        "driver": {"editing_cpu_accounting": enabled, "native_trace": False,
                                   "edit_interval_ms": 5, "settled_idle_seconds": 3,
                                   "settled_idle_location": "startup"},
                        "provenance": {"recorded_at_unix": repetition + int(enabled),
                                       "harness_dirty": False, "binary_sha256": "binary",
                                       "harness_revision": "harness", "environment": {}, "files": {}},
                        "checkpoints": [],
                    }
                    events = []
                    for cycle in range(40):
                        timing = {"recovery_completion_ms": 2, "cancelled_rename_ms": 1,
                                  "repair_to_rename_send_ms": 0.1, "edit_intervals_ms": [5]}
                        if enabled:
                            timing["editing_server_cpu_ms"] = 10
                        ids = {"rename": cycle * 2, "completion": cycle * 2 + 1}
                        report["checkpoints"].append({"cycle": cycle, "timing": timing,
                            "result_sha256": "result", "fresh_oracle_matched": True,
                            "recovery_ms": 5, "recovery_requests": ids})
                        for key in ids.values():
                            events.extend([{"event": "send", "pid": repetition, "id": key, "started_ns": 1000},
                                           {"event": "response", "pid": repetition, "id": key, "received_ns": 1_001_000}])
                    label = "enabled" if enabled else "disabled"
                    path = root / f"{label}-{repetition}.json"
                    path.write_text(json.dumps(report))
                    path.with_suffix(".jsonl").write_text("\n".join(json.dumps(event) for event in events))
            result = accounting.summarize(root)
            self.assertEqual(result["pairs"][0]["disabled"]["completion_wire_ms"]["p95"], 1)
            path = root / "enabled-1.json"
            original = path.read_text()
            for corruption in ("binary", "flag", "idle", "dirty", "parity", "oracle", "missing_cpu"):
                report = json.loads(original)
                if corruption == "binary":
                    report["provenance"]["binary_sha256"] = "other"
                elif corruption == "flag":
                    report["driver"]["editing_cpu_accounting"] = False
                elif corruption == "idle":
                    report["driver"]["settled_idle_location"] = "after_cycles"
                elif corruption == "dirty":
                    report["provenance"]["harness_dirty"] = True
                elif corruption == "parity":
                    report["checkpoints"][-1]["result_sha256"] = "other"
                elif corruption == "oracle":
                    report["checkpoints"][-1]["fresh_oracle_matched"] = False
                else:
                    del report["checkpoints"][-1]["timing"]["editing_server_cpu_ms"]
                path.write_text(json.dumps(report))
                with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                    accounting.summarize(root)
