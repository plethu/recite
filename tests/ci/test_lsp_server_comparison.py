"""Server comparisons must preserve results and reject incomparable evidence."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

scripts = Path(__file__).resolve().parents[2] / "scripts"
sys.path.insert(0, str(scripts))
spec = importlib.util.spec_from_file_location("server_comparison", scripts / "summarize-lsp-server-comparison.py")
comparison = importlib.util.module_from_spec(spec)
spec.loader.exec_module(comparison)

from lsp_session_workload import Session


class ServerComparisonTests(unittest.TestCase):
    def test_checkpoint_fingerprints_preserve_relative_target_identity(self):
        def checkpoint(root, target):
            session = Session.__new__(Session)
            session.root = root
            session.errors = {}
            session.completion = session.reference = {}
            session.query = lambda *_: (1, {"uri": (root / target).as_uri(), "label": "scene"})
            session.collect_diagnostics = lambda: None
            return session.checkpoint()["result_sha256"]

        with tempfile.TemporaryDirectory(prefix="recite comparison ") as directory:
            root = Path(directory).resolve()
            a = checkpoint(root / "one", "shard.recite")
            b = checkpoint(root / "two", "shard.recite")
            self.assertEqual(a, b)
            self.assertNotEqual(a, checkpoint(root / "two", "other.recite"))

    def test_comparison_rejects_missing_parity_or_mismatched_driver(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for side in (root, root / "control"):
                side.mkdir(exist_ok=True)
                for mode in ("fixed", "churn"):
                    for repetition in range(1, 4):
                        report = {
                            "status": "pass", "health": {"status": "pass"}, "cycles": 40,
                            "edits_per_cycle": 50, "churn": mode == "churn", "seed": 7203,
                            "driver": {"native_trace": False}, "result_fingerprint_version": 2,
                            "provenance": {"binary_sha256": str(side), "harness_revision": "harness",
                                           "files": {"source": "text"}, "environment": {}},
                            "checkpoints": [{"cycle": index, "recovery_ms": 10.0,
                                             "timing": {"server_cpu_ms": 100}, "result_sha256": "result",
                                             "fresh_oracle_matched": True} for index in range(40)],
                        }
                        (side / f"{mode}-{repetition}.json").write_text(json.dumps(report))
            self.assertEqual(comparison.summarize(root)["changes"]["recovery_ms"]["median"]["percent"], 0)
            path = root / "fixed-1.json"
            original = path.read_text()
            for corruption in ("parity", "oracle", "driver", "pacing", "version", "cpu", "editing_partial", "idle_partial", "incomplete"):
                report = json.loads(original)
                if corruption == "parity":
                    report["checkpoints"][6]["result_sha256"] = "different"
                elif corruption == "oracle":
                    report["checkpoints"][-1]["fresh_oracle_matched"] = False
                elif corruption == "driver":
                    report["driver"]["native_trace"] = True
                elif corruption == "pacing":
                    report["driver"]["edit_interval_ms"] = 250
                elif corruption == "version":
                    report["result_fingerprint_version"] = 1
                elif corruption == "cpu":
                    report["checkpoints"][-1]["timing"]["server_cpu_ms"] = float("nan")
                elif corruption == "editing_partial":
                    report["checkpoints"][-1]["timing"]["editing_server_cpu_ms"] = 10
                elif corruption == "idle_partial":
                    report["settled_idle"] = {"elapsed_ms": 3000, "server_cpu_ms": 0}
                else:
                    report["checkpoints"].pop()
                path.write_text(json.dumps(report))
                with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                    comparison.summarize(root)
            path.write_text(original)
            for report_path in root.rglob("*.json"):
                report = json.loads(report_path.read_text())
                report["cycles"] = 20
                report["edits_per_cycle"] = 10
                report["driver"]["edit_interval_ms"] = 100
                report["checkpoints"] = report["checkpoints"][:20]
                report_path.write_text(json.dumps(report))
            # An experimental profile cannot silently satisfy the default CI contract.
            with self.assertRaises(ValueError):
                comparison.summarize(root)
            self.assertEqual(comparison.summarize(root, cycles=20, edits=10)["changes"]["recovery_ms"]["median"]["percent"], 0)
