"""Regression decisions must reject broken evidence and tolerate isolated noise."""

import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("regression", ROOT / "scripts/lsp_regression.py")
regression = importlib.util.module_from_spec(spec)
spec.loader.exec_module(regression)
spec = importlib.util.spec_from_file_location("measurement", ROOT / "scripts/lsp_measurement.py")
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)


def pairs():
    result = {"files": {"a.recite": "same"}, "workloads": {"prose": [10.0] * 21},
              "diagnostics": {"prose": [{"count": 0, "sha256": "empty"}] * 21}}
    return [{"control": copy.deepcopy(result), "candidate": copy.deepcopy(result)} for _ in range(3)]


class RegressionTests(unittest.TestCase):
    def test_range_probe_preserves_unicode_and_line_endings(self):
        event = measurement.change_event("a\r\n💬x\r\n", "a\r\n💬z\r\n", True)
        self.assertEqual(event, {"range": {"start": {"line": 1, "character": 2},
                                          "end": {"line": 1, "character": 3}}, "text": "z"})
        event = measurement.change_event("a\r\nb", "a\nb", True)
        self.assertEqual(event, {"range": {"start": {"line": 0, "character": 1},
                                          "end": {"line": 1, "character": 0}}, "text": "\n"})
        event = measurement.change_event("a\rb", "a\rc", True)
        self.assertEqual(event["range"]["start"], {"line": 1, "character": 0})

    def test_fanout_fixture_identity_is_required_and_must_match(self):
        observations = pairs()
        for pair in observations:
            for result in pair.values():
                result["workloads"]["fanout/block_topology"] = [10.0] * 21
                result["diagnostics"]["fanout/block_topology"] = [{"sha256": "empty"}] * 21
        coverage = {"prose", "fanout/block_topology"}
        with self.assertRaises(ValueError):
            regression.regressions(observations, 1.2, 2, coverage)
        for pair in observations:
            for result in pair.values():
                result["fanout_fixture_files"] = {"src/shared.recite": "same"}
        self.assertEqual(regression.regressions(observations, 1.2, 2, coverage), [])
        observations[0]["candidate"]["fanout_fixture_files"] = {"src/shared.recite": "different"}
        with self.assertRaises(ValueError):
            regression.regressions(observations, 1.2, 2, coverage)

    def test_memory_threshold_requires_relative_and_absolute_growth(self):
        observations = pairs()
        limits = {"prose": {"ratio": 1.2, "absolute": 16384, "unit": "KiB"}}
        for pair in observations:
            pair["control"]["workloads"]["prose"] = [100_000] * 21
            pair["candidate"]["workloads"]["prose"] = [130_000] * 21
        finding = regression.regressions(observations, 1.2, 2, {"prose"}, limits)[0]
        self.assertEqual(finding["unit"], "KiB")
        self.assertEqual(finding["candidate"], 130_000)
        for pair in observations:
            pair["candidate"]["workloads"]["prose"] = [115_000] * 21
        self.assertEqual(regression.regressions(observations, 1.2, 2, {"prose"}, limits), [])
        with self.assertRaises(ValueError):
            regression.regressions(observations, 1.2, 2, {"prose"}, {"missing": limits["prose"]})

    def test_identical_runs_pass(self):
        self.assertEqual(regression.regressions(pairs(), 1.2, 2, {"prose"}), [])

    def test_missing_workload_on_both_sides_fails(self):
        with self.assertRaises(ValueError):
            regression.regressions(pairs(), 1.2, 2, {"prose", "completion"})

    def test_injected_slowdown_fails(self):
        observations = pairs()
        for pair in observations:
            pair["candidate"]["workloads"]["prose"] = [15.0] * 21
        self.assertEqual(regression.regressions(observations, 1.2, 2, {"prose"})[0]["workload"], "prose")

    def test_one_noisy_pair_does_not_fail(self):
        observations = pairs()
        observations[0]["candidate"]["workloads"]["prose"] = [100.0] * 21
        self.assertEqual(regression.regressions(observations, 1.2, 2, {"prose"}), [])

    def test_absolute_tolerance_protects_submillisecond_noise(self):
        observations = pairs()
        for pair in observations:
            pair["control"]["workloads"]["prose"] = [0.1] * 21
            pair["candidate"]["workloads"]["prose"] = [0.2] * 21
        self.assertEqual(regression.regressions(observations, 1.2, 2, {"prose"}), [])

    def test_invalid_evidence_fails_closed(self):
        for corruption in ("diagnostics", "files", "samples", "nan", "coverage"):
            observations = pairs()
            candidate = observations[0]["candidate"]
            if corruption == "diagnostics":
                candidate["diagnostics"]["prose"][0] = {"count": 1, "sha256": "different"}
            elif corruption == "files":
                candidate["files"]["a.recite"] = "changed"
            elif corruption == "samples":
                candidate["workloads"]["prose"] = [10.0]
            elif corruption == "nan":
                candidate["workloads"]["prose"][0] = float("nan")
            else:
                del candidate["workloads"]["prose"]
            with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                regression.regressions(observations, 1.2, 2, {"prose"})


if __name__ == "__main__":
    unittest.main()
