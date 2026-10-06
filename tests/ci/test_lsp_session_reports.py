"""Incomplete or incomparable session reports must not pass the tail gate."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

scripts = Path(__file__).resolve().parents[2] / "scripts"
sys.path.insert(0, str(scripts))
spec = importlib.util.spec_from_file_location("session_recovery", scripts / "check-lsp-session-recovery.py")
recovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recovery)


class SessionReportsTests(unittest.TestCase):
    def test_idle_cpu_rejects_repeated_spinning_and_invalid_accounting(self):
        intervals = [{"elapsed_ms": 3000, "server_cpu_ms": value} for value in (150, 150, 0)]
        self.assertEqual(recovery.idle_cpu_budget(intervals, 100)["status"], "regression")
        intervals[1]["server_cpu_ms"] = 0
        self.assertEqual(recovery.idle_cpu_budget(intervals, 100)["status"], "pass")
        for key, value in (("elapsed_ms", 0), ("elapsed_ms", float("nan")),
                           ("server_cpu_ms", -1), ("server_cpu_ms", float("inf"))):
            invalid = [dict(row) for row in intervals]
            invalid[0][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                recovery.idle_cpu_budget(invalid, 100)
        with self.assertRaises(ValueError):
            recovery.idle_cpu_budget(intervals[:2], 100)

    def test_completion_tail_cannot_hide_inside_the_total_recovery_budget(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for mode in ("fixed", "churn"):
                for repetition in range(1, 4):
                    report = {"status": "pass", "health": {"status": "pass"}, "cycles": 40,
                              "edits_per_cycle": 50, "churn": mode == "churn", "seed": 7203,
                              "provenance": {"binary_sha256": "binary", "harness_revision": "harness",
                                             "files": {"source": "text"}},
                              "checkpoints": [{"cycle": index, "recovery_ms": 20.0,
                                               "timing": {"recovery_completion_ms": 10.0 if repetition < 3 else 0.5}}
                                              for index in range(40)]}
                    (root / f"{mode}-{repetition}.json").write_text(json.dumps(report))
            self.assertEqual(recovery.evaluate(root, 75)["status"], "pass")
            self.assertEqual(recovery.evaluate(root, 75, 5)["status"], "regression")
            for mode in ("fixed", "churn"):
                path = root / f"{mode}-2.json"
                report = json.loads(path.read_text())
                for row in report["checkpoints"]:
                    row["timing"]["recovery_completion_ms"] = 0.5
                path.write_text(json.dumps(report))
            self.assertEqual(recovery.evaluate(root, 75, 5)["status"], "pass")
            path = root / "fixed-3.json"
            original = path.read_text()
            for corruption in (None, float("nan"), -1):
                report = json.loads(original)
                if corruption is None:
                    report["checkpoints"][-1]["timing"].clear()
                else:
                    report["checkpoints"][-1]["timing"]["recovery_completion_ms"] = corruption
                path.write_text(json.dumps(report))
                with self.subTest(corruption=corruption), self.assertRaises((KeyError, ValueError)):
                    recovery.evaluate(root, 75, 5)

    def test_missing_truncated_and_mismatched_reports_fail_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                recovery.evaluate(root, 25)
            for mode in ("fixed", "churn"):
                for repetition in range(1, 4):
                    report = {"status": "pass", "health": {"status": "pass"}, "cycles": 40,
                              "edits_per_cycle": 50, "churn": mode == "churn", "seed": 7203,
                              "provenance": {"binary_sha256": "binary", "harness_revision": "harness",
                                             "files": {"source": "text"}},
                              "checkpoints": [{"cycle": index, "recovery_ms": 10.0} for index in range(40)]}
                    (root / f"{mode}-{repetition}.json").write_text(json.dumps(report))
            self.assertEqual(recovery.evaluate(root, 25)["status"], "pass")
            path = root / "churn-3.json"
            original = path.read_text()
            for corruption in ("truncated", "binary", "harness", "seed", "driver", "nan", "error", "failed"):
                report = json.loads(original)
                if corruption == "truncated":
                    report["checkpoints"].pop()
                elif corruption == "binary":
                    report["provenance"]["binary_sha256"] = "other"
                elif corruption == "harness":
                    report["provenance"]["harness_revision"] = "other"
                elif corruption == "seed":
                    report["seed"] += 1
                elif corruption == "driver":
                    report["driver"] = {"switch_interval_ms": 2}
                elif corruption == "nan":
                    report["checkpoints"][-1]["recovery_ms"] = float("nan")
                elif corruption == "error":
                    report["error"] = "shutdown failed after the last checkpoint"
                else:
                    report["status"] = "regression"
                path.write_text(json.dumps(report))
                with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                    recovery.evaluate(root, 25)
