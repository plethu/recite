"""Regression decisions must reject broken evidence and tolerate isolated noise."""

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.lsp_tools import measurement, performance, regression


def pairs():
    result = {
        "files": {"a.recite": "same"},
        "workloads": {"prose": [10.0] * 21},
        "diagnostics": {"prose": [{"count": 0, "sha256": "empty"}] * 21},
    }
    return [
        {"control": copy.deepcopy(result), "candidate": copy.deepcopy(result)} for _ in range(3)
    ]


class RegressionTests(unittest.TestCase):
    def test_range_probe_preserves_unicode_and_line_endings(self):
        event = measurement.change_event("a\r\n💬x\r\n", "a\r\n💬z\r\n", True)
        self.assertEqual(
            event,
            {
                "range": {"start": {"line": 1, "character": 2}, "end": {"line": 1, "character": 3}},
                "text": "z",
            },
        )
        event = measurement.change_event("a\r\nb", "a\nb", True)
        self.assertEqual(
            event,
            {
                "range": {"start": {"line": 0, "character": 1}, "end": {"line": 1, "character": 0}},
                "text": "\n",
            },
        )
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
        self.assertEqual(
            regression.regressions(observations, 1.2, 2, {"prose"})[0]["workload"], "prose"
        )

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


class ConfirmationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        directory = Path(temporary.name)
        self.root = directory / "project"
        self.root.mkdir()
        (self.root / "scene.recite").write_text(":: start\n-> END\n")
        self.control, self.candidate = directory / "control", directory / "candidate"
        self.control.write_bytes(b"control")
        self.candidate.write_bytes(b"candidate")
        self.output = directory / "report.json"
        self.policy = json.loads(Path("scripts/lsp-performance-policy.json").read_text())
        self.calls = []

    def compare(self, slow_rounds, *, interrupt=False, corrupt=False, mutate=False):
        def measure(binary, root, policy, workloads):
            round_index = len(self.calls) // 6
            if interrupt and round_index == 1:
                raise KeyboardInterrupt("interrupted confirmation")
            self.calls.append((binary.name, list(workloads), policy["samples"]))
            if mutate and round_index == 1:
                (root / "scene.recite").write_text("changed on both sides")
            values = {}
            for name in workloads:
                baseline = 100_000.0 if name.startswith("memory/") else 100.0
                slow = binary == self.candidate and name in slow_rounds[round_index]
                values[name] = [baseline * (1.6 if slow else 1)] * policy["samples"]
            result = {
                "files": measurement.fixture_files(root),
                "workloads": values,
                "diagnostics": {name: [{"sha256": "same"}] * policy["samples"] for name in values},
            }
            if "fanout/block_topology" in workloads:
                result["fanout_fixture_files"] = {"scene.recite": "same"}
            if corrupt and round_index == 1 and binary == self.candidate:
                result["files"]["scene.recite"] = "changed"
            return result

        with patch.object(performance, "measure", side_effect=measure):
            return performance.compare(
                self.control, self.candidate, self.root, self.output, self.policy
            )

    def test_clean_round_does_not_repeat(self):
        self.assertTrue(self.compare([set()]))
        self.assertEqual(len(self.calls), 6)
        self.assertTrue(all(coverage == self.policy["workloads"] for _, coverage, _ in self.calls))
        self.assertEqual(
            [side for side, _, _ in self.calls],
            ["control", "candidate", "candidate", "control", "control", "candidate"],
        )

    def test_fix_all_confirmation_repeats_the_complete_query_family(self):
        finding = {"query/code_action_fix_all"}
        self.assertFalse(self.compare([finding, finding]))
        queries = [name for name in self.policy["workloads"] if name.startswith("query/")]
        self.assertEqual(len(self.calls), 12)
        self.assertTrue(
            all(coverage == queries and samples == 21 for _, coverage, samples in self.calls[6:])
        )
        self.assertEqual(
            [side for side, _, _ in self.calls[6:]],
            ["candidate", "control", "control", "candidate", "candidate", "control"],
        )
        report = json.loads(self.output.read_text())
        self.assertEqual(report["status"], "regression")
        self.assertEqual(report["rounds"][1]["suspected_workloads"], sorted(finding))

    def test_confirmation_unions_families_and_preserves_startup_limits(self):
        findings = {"query/code_action_fix_all", "lifecycle/index_ready"}
        self.assertFalse(self.compare([findings, findings]))
        expected = [
            name
            for name in self.policy["workloads"]
            if name.startswith(("query/", "lifecycle/", "memory/"))
        ]
        self.assertTrue(all(coverage == expected for _, coverage, _ in self.calls[6:]))
        report = json.loads(self.output.read_text())
        self.assertEqual(report["policy"]["limits"], self.policy["limits"])
        self.assertEqual({row["workload"] for row in report["rounds"][1]["regressions"]}, findings)

    def test_cleared_suspicion_passes_but_different_slowdown_is_unstable(self):
        finding = {"query/code_action_fix_all"}
        self.assertTrue(self.compare([finding, set()]))
        self.calls.clear()
        self.assertFalse(self.compare([finding, {"query/hover"}]))
        self.assertEqual(json.loads(self.output.read_text())["status"], "unstable")

    def test_interruption_and_changed_confirmation_fixtures_fail_closed(self):
        finding = {"query/code_action_fix_all"}
        with self.assertRaises(KeyboardInterrupt):
            self.compare([finding, finding], interrupt=True)
        self.assertEqual(json.loads(self.output.read_text())["status"], "incomplete")
        self.calls.clear()
        with self.assertRaises(ValueError):
            self.compare([finding, finding], corrupt=True)
        self.assertEqual(json.loads(self.output.read_text())["status"], "incomplete")

    def test_matching_but_changed_confirmation_fixtures_fail_closed(self):
        finding = {"query/code_action_fix_all"}
        with self.assertRaises(ValueError):
            self.compare([finding, finding], mutate=True)
        self.assertEqual(json.loads(self.output.read_text())["status"], "incomplete")

    def test_query_measurement_keeps_probe_sequence_and_checks_live_fixture_identity(self):
        queries = [name for name in self.policy["workloads"] if name.startswith("query/")]
        interactive = {
            "requests": {
                name.removeprefix("query/"): [
                    {"ms": 10.0, "result_sha256": "same", "result_bytes": 1}
                ]
                * 21
                for name in queries
            }
        }
        with (
            patch.object(performance.latency, "run", return_value=interactive) as query,
            patch.object(performance.edits, "measure") as edit,
            patch.object(performance, "measure_lifecycle") as startup,
            patch.object(performance, "generate_fanout") as fanout,
        ):
            result = performance.measure(self.control, self.root, self.policy, queries)
            self.assertEqual(set(result["workloads"]), set(queries))
            query.assert_called_once()
            edit.assert_not_called()
            startup.assert_not_called()
            fanout.assert_not_called()
            self.assertEqual(result["files"], measurement.fixture_files(self.root))

            def mutate_fixture(*_):
                (self.root / "scene.recite").write_text("changed")
                return interactive

            query.side_effect = mutate_fixture
            with self.assertRaises(RuntimeError):
                performance.measure(self.control, self.root, self.policy, queries)


if __name__ == "__main__":
    unittest.main()
