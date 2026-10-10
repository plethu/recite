"""An intentional skip is distinct from lost, failed or cancelled coverage."""

import copy
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import textwrap
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
        installed = re.findall(r"install_args: ([^\n]+)", jobs["git-policy"])
        self.assertTrue({"just", "aqua:jqlang/jq"} <= set(" ".join(installed).split()))
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


class MutationCommandTests(unittest.TestCase):
    critical_files = ("crates/first/src/critical.rs", "crates/second/src/nested/critical.rs")

    def run_recipe(self, inventory, *args, metadata=None, baseline_exit=0):
        if metadata is None:
            metadata = {"metadata": {"recite": {"critical_mutation_files": self.critical_files}}}
        with tempfile.TemporaryDirectory(prefix="recite mutation recipe ") as directory:
            root = Path(directory)
            command_dir = root / "bin"
            command_dir.mkdir()
            cargo = command_dir / "cargo"
            cargo.write_text(
                f"#!{sys.executable}\n"
                + textwrap.dedent(
                    """\
                    import json
                    import sys
                    from pathlib import Path

                    args = sys.argv[1:]
                    with Path("calls.jsonl").open("a") as log:
                        log.write(json.dumps(args) + "\\n")
                    if args[0] == "metadata":
                        print(Path("metadata.json").read_text())
                    elif args[0] == "mutants" and "--list" in args:
                        print(Path("inventory.json").read_text())
                    elif args[0] == "nextest":
                        sys.exit(int(Path("baseline-exit").read_text()))
                    elif args[0] != "mutants":
                        sys.exit("unexpected Cargo command: " + repr(args))
                    """
                )
            )
            cargo.chmod(0o755)
            (root / "metadata.json").write_text(json.dumps(metadata))
            (root / "inventory.json").write_text(json.dumps(inventory))
            (root / "baseline-exit").write_text(str(baseline_exit))
            completed = subprocess.run(
                [
                    "just",
                    "--justfile",
                    str(ROOT / "justfile"),
                    "--working-directory",
                    str(root),
                    "mutants",
                    *args,
                ],
                env=os.environ
                | {
                    "PATH": f"{command_dir}{os.pathsep}{os.environ['PATH']}",
                    "XDG_RUNTIME_DIR": str(root),
                },
                text=True,
                capture_output=True,
                timeout=15,
                check=False,
            )
            log = root / "calls.jsonl"
            calls = (
                [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []
            )
        return completed, calls

    def test_valid_shard_keeps_complete_preflight_and_execution_scope(self):
        controls = ("--shard", "1/2", "--jobs", "2", "--build-timeout", "600")
        inventory = [{"file": path} for path in self.critical_files]
        completed, calls = self.run_recipe(inventory, *controls)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        scope = [value for path in self.critical_files for value in ("--file", path)]
        self.assertEqual(
            calls,
            [
                ["metadata", "--locked", "--no-deps", "--format-version", "1"],
                ["mutants", "--workspace", "--list", "--json", *scope],
                [
                    "nextest",
                    "run",
                    "--locked",
                    "-p",
                    "recite-core",
                    "-p",
                    "recite-compiler",
                    "-p",
                    "recite-lsp",
                    "-p",
                    "recite-runtime",
                    "--cargo-profile",
                    "mutants",
                    "--test-threads",
                    "4",
                ],
                ["mutants", "--workspace", "--output", "target/mutation", *scope, *controls],
            ],
        )

    def test_empty_missing_stale_or_partial_inventory_stops_before_baseline(self):
        for inventory in (
            [],
            None,
            [{"name": "candidate without a file"}],
            [{"file": "crates/removed/src/critical.rs"}],
            [{"file": self.critical_files[0]}],
        ):
            with self.subTest(inventory=inventory):
                completed, calls = self.run_recipe(inventory)
                self.assertNotEqual(completed.returncode, 0, completed.stdout)
                self.assertEqual([call[0] for call in calls], ["metadata", "mutants"])
                self.assertIn("--list", calls[1])

    def test_missing_or_empty_scope_stops_before_inventory(self):
        for metadata in ({}, {"metadata": {"recite": {"critical_mutation_files": []}}}):
            with self.subTest(metadata=metadata):
                completed, calls = self.run_recipe([], metadata=metadata)
                self.assertNotEqual(completed.returncode, 0, completed.stdout)
                self.assertEqual([call[0] for call in calls], ["metadata"])

    def test_scope_narrowing_and_baseline_skipping_are_rejected_before_cargo(self):
        inventory = [{"file": path} for path in self.critical_files]
        for args in (
            ("--file", self.critical_files[0]),
            ("--re", "only_one_mutation"),
            ("--exclude", self.critical_files[0]),
            ("--baseline", "skip"),
            ("--baseline=skip",),
            ("--config", "different.toml"),
            ("--test-package", "recite-core"),
            ("--no-config",),
            ("--iterate",),
        ):
            with self.subTest(args=args):
                completed, calls = self.run_recipe(inventory, *args)
                self.assertNotEqual(completed.returncode, 0, completed.stdout)
                self.assertIn("just mutants accepts only", completed.stderr)
                self.assertEqual(calls, [])

    def test_failed_downstream_baseline_stops_mutation_execution(self):
        inventory = [{"file": path} for path in self.critical_files]
        completed, calls = self.run_recipe(inventory, baseline_exit=42)
        self.assertNotEqual(completed.returncode, 0, completed.stdout)
        self.assertEqual([call[0] for call in calls], ["metadata", "mutants", "nextest"])
        self.assertIn("--list", calls[1])


if __name__ == "__main__":
    unittest.main()
