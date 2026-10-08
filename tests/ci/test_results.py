"""An intentional skip is distinct from lost, failed or cancelled coverage."""

import copy
import importlib.util
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("ci_results", ROOT / "scripts/check-ci-results.py")
results = importlib.util.module_from_spec(spec)
spec.loader.exec_module(results)


def fixture():
    needs = {lane: {"result": "skipped"} for lane in results.LANES}
    needs["docs"]["result"] = "success"
    needs["changes"] = {
        "result": "success",
        "outputs": {lane: "true" if lane == "docs" else "false" for lane in results.LANES},
    }
    needs["git-policy"] = {"result": "success"}
    return needs


class ResultsTests(unittest.TestCase):
    def test_intentional_skips_and_full_success(self):
        self.assertEqual(results.failures(fixture()), [])
        needs = fixture()
        for lane in results.LANES:
            needs["changes"]["outputs"][lane] = "true"
            needs[lane]["result"] = "success"
        self.assertEqual(results.failures(needs), [])

    def test_selected_lane_must_succeed(self):
        for lane in results.LANES:
            for result in ("skipped", "failure", "cancelled", None):
                needs = fixture()
                needs["changes"]["outputs"][lane] = "true"
                needs[lane]["result"] = result
                with self.subTest(lane=lane, result=result):
                    self.assertIn(
                        f"{lane}: expected success, got {result}", results.failures(needs)
                    )

    def test_site_browser_failure_blocks_without_native_jobs(self):
        needs = fixture()
        needs["site"]["result"] = "failure"
        needs["changes"]["outputs"]["site"] = "true"
        self.assertEqual(results.failures(needs), ["site: expected success, got failure"])

    def test_unselected_failure_is_not_hidden(self):
        for lane in results.LANES:
            for result in ("failure", "cancelled", "success"):
                needs = fixture()
                needs["changes"]["outputs"][lane] = "false"
                needs[lane]["result"] = result
                with self.subTest(lane=lane, result=result):
                    self.assertIn(
                        f"{lane}: expected skipped, got {result}", results.failures(needs)
                    )

    def test_missing_invalid_or_failed_selection_blocks(self):
        original = fixture()
        for job in original:
            needs = copy.deepcopy(original)
            del needs[job]
            self.assertTrue(results.failures(needs))
        for result in ("failure", "cancelled", "skipped"):
            for job in ("changes", "git-policy"):
                needs = fixture()
                needs[job]["result"] = result
                self.assertTrue(results.failures(needs))
        for value in (None, "", "yes", True, False, [], {}):
            needs = fixture()
            needs["changes"]["outputs"]["rust"] = value
            self.assertTrue(results.failures(needs))
        needs = fixture()
        needs["changes"]["outputs"]["new-lane"] = "false"
        self.assertTrue(results.failures(needs))
        needs = fixture()
        needs["new-lane"] = {"result": "success"}
        self.assertIn("required jobs are missing or unexpected", results.failures(needs))

    def test_malformed_job_and_selection_objects_fail_closed(self):
        for value in (None, [], "success"):
            self.assertTrue(results.failures(value))
            for lane in results.LANES | {"changes", "git-policy"}:
                needs = fixture()
                needs[lane] = value
                with self.subTest(lane=lane, value=value):
                    self.assertTrue(results.failures(needs))
            needs = fixture()
            needs["changes"]["outputs"] = value
            self.assertTrue(results.failures(needs))

    def test_workflow_wires_every_lane_to_selection_and_required_result(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        job_pattern = re.compile(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)", re.M | re.S)
        jobs = dict(job_pattern.findall(workflow.split("jobs:\n", 1)[1]))
        self.assertEqual(set(jobs), results.LANES | {"changes", "git-policy", "required-check"})
        for lane in results.LANES:
            self.assertIn("needs: changes", jobs[lane])
            self.assertIn(f"if: needs.changes.outputs.{lane} == 'true'", jobs[lane])
            self.assertIn(f"steps.scope.outputs.{lane}", jobs["changes"])
            self.assertIn(f"      - {lane}\n", jobs["required-check"])
        self.assertIn("if: always()", jobs["required-check"])
        self.assertIn("${{ toJSON(needs) }}", jobs["required-check"])
        self.assertIn("scripts/check-ci-results.py", jobs["required-check"])
        self.assertNotIn("      - edited", workflow)
        self.assertNotIn("      - labeled", workflow)
        self.assertNotIn("      - unlabeled", workflow)
        self.assertEqual(workflow.count("uses: ./.github/workflows/cli-packages.yml"), 1)
        self.assertIn("uses: ./.github/workflows/cli-packages.yml", jobs["cli-packages"])
        packages = (ROOT / ".github/workflows/writer-packages.yml").read_text()
        self.assertIn("  workflow_call:", packages)
        self.assertIn("  workflow_dispatch:", packages)
        self.assertNotIn("  pull_request:", packages)
        package_jobs = dict(job_pattern.findall(packages.split("jobs:\n", 1)[1]))
        families = {"packages": "native", "nix-packages": "nix", "flatpak-packages": "flatpak"}
        for lane, selected_family in families.items():
            for family in families.values():
                expected = "true" if family == selected_family else "false"
                self.assertIn(f"      {family}: {expected}\n", jobs[lane])
            # Called workflows inherit the caller event: dispatch cannot override false inputs.
            job = "package" if selected_family == "native" else selected_family
            self.assertIn(f"    if: inputs.{selected_family}\n", package_jobs[job])


if __name__ == "__main__":
    unittest.main()
