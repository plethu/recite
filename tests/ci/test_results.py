"""An intentional skip is distinct from lost, failed or cancelled coverage."""

import copy
import importlib.util
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ci_results", ROOT / "scripts/check-ci-results.py")
results = importlib.util.module_from_spec(spec)
spec.loader.exec_module(results)


def fixture():
    needs = {lane: {"result": "skipped"} for lane in results.LANES}
    needs["docs"]["result"] = "success"
    needs["changes"] = {"result": "success", "outputs": {
        lane: "true" if lane == "docs" else "false" for lane in results.LANES
    }}
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
        for result in ("skipped", "failure", "cancelled", None):
            needs = fixture()
            needs["docs"]["result"] = result
            with self.subTest(result=result):
                self.assertTrue(results.failures(needs))

    def test_unselected_failure_is_not_hidden(self):
        for result in ("failure", "cancelled", "success"):
            needs = fixture()
            needs["rust"]["result"] = result
            self.assertTrue(results.failures(needs))

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
        for value in (None, "", "yes", True):
            needs = fixture()
            needs["changes"]["outputs"]["rust"] = value
            self.assertTrue(results.failures(needs))
        needs = fixture()
        needs["changes"]["outputs"]["new-lane"] = "false"
        self.assertTrue(results.failures(needs))

    def test_workflow_wires_every_lane_to_selection_and_required_result(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        jobs = dict(re.findall(r"^  ([\w-]+):\n(.*?)(?=^  [\w-]+:|\Z)",
                               workflow.split("jobs:\n", 1)[1], re.M | re.S))
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
        packages = (ROOT / ".github/workflows/writer-packages.yml").read_text()
        self.assertIn("  workflow_call:", packages)
        self.assertIn("  workflow_dispatch:", packages)
        self.assertNotIn("  pull_request:", packages)


if __name__ == "__main__":
    unittest.main()
