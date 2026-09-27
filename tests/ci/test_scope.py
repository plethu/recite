"""Coverage selection contracts, including real Git rename and merge-base diffs."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("ci_scope", ROOT / "scripts/ci-scope.py")
scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scope)


def selected(*paths):
    return {lane for lane, run in scope.select_lanes(paths).items() if run}


class ScopeTests(unittest.TestCase):
    def test_documentation_avoids_builds(self):
        for path in ("docs/recite-production-spec.md", "README.md", "apps/writer/acceptance.md",
                     "apps/writer/packaging/README.md", "docs-site/src/content/docs/index.md"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs"})

    def test_maintainability_exceptions_select_policy_and_docs(self):
        self.assertEqual(selected("scripts/maintainability/exceptions.toml"),
                         {"docs", "maintainability"})

    def test_core_changes_keep_windows_and_benchmarks(self):
        self.assertEqual(selected("crates/recite-runtime/src/lib.rs"), {
            "rust", "windows-publisher", "benchmark-smoke", "editor", "maintainability",
        })

    def test_writer_keeps_ui_and_accessibility_without_packaging(self):
        self.assertEqual(selected("apps/writer/crates/freya/src/app.rs"), {
            "rust", "maintainability",
        })
        self.assertIn("rust", selected("scripts/check-writer-native-accessibility.py"))

    def test_editor_and_schema_consumers(self):
        self.assertIn("editor", selected("crates/recite-lsp/src/lib.rs"))
        self.assertEqual(selected("editors/vscode/src/extension.ts"), {"editor", "maintainability"})
        self.assertTrue({"docs", "rust", "editor"} <= selected("schemas/manifest.json"))
        self.assertTrue({"docs", "rust", "editor"} <= selected("fixtures/schema/valid/test.json"))
        self.assertIn("rust", selected("fixtures/recite/markdown-input.md"))

    def test_packaging_and_shared_build_inputs(self):
        for path in (
            "flake.nix", "apps/writer/packaging/flatpak/manifest.json",
            "scripts/package-writer.py", "assets/identity/recite.png",
            "apps/writer/Cargo.lock", "crates/recite-core/Cargo.toml",
        ):
            with self.subTest(path=path):
                self.assertIn("packages", selected(path))
        for path in ("Cargo.lock", ".mise.toml", "justfile", ".github/workflows/ci.yml"):
            self.assertEqual(selected(path), scope.LANES)

    def test_unknown_inputs_are_conservative_and_changes_union(self):
        self.assertEqual(selected("new-build-system/config.json"), scope.LANES)
        self.assertEqual(selected("docs/recite-production-spec.md", "crates/recite-cli/src/lib.rs"),
                         {"docs"} | scope.RUST)
        self.assertEqual(selected(), set())

    def test_full_events_and_initial_push(self):
        for event in ("schedule", "workflow_dispatch"):
            self.assertTrue(all(scope.event_scope(event, {}).values()))
        self.assertTrue(all(scope.event_scope("push", {"before": "0" * 40, "after": "head"}).values()))
        with self.assertRaises(ValueError):
            scope.event_scope("pull_request_target", {})
        with self.assertRaises(KeyError):
            scope.event_scope("pull_request", {})

    def test_event_uses_pr_heads_and_direct_push_range(self):
        with patch.object(scope, "changed_paths", return_value=["README.md"]) as diff:
            scope.event_scope("pull_request", {"pull_request": {
                "base": {"sha": "base"}, "head": {"sha": "head"},
            }})
            diff.assert_called_once_with("base", "head", pull_request=True)
        with patch.object(scope, "changed_paths", return_value=[]) as diff:
            scope.event_scope("push", {"before": "old", "after": "new"})
            diff.assert_called_once_with("old", "new", pull_request=False)


class GitDiffTests(unittest.TestCase):
    def test_complete_diff_includes_deletions_rename_sources_and_unusual_names(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args):
                return subprocess.check_output(["git", "-C", directory, *args], text=True).strip()

            git("init", "--quiet", "--initial-branch=main")
            git("config", "user.name", "CI fixtures")
            git("config", "user.email", "ci@example.invalid")
            git("config", "commit.gpgsign", "false")
            (root / "crates").mkdir()
            (root / "crates/old.rs").write_text("fn old() {}\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "base")
            base = git("rev-parse", "HEAD")
            git("switch", "--quiet", "-c", "topic")
            (root / "docs").mkdir()
            (root / "crates/old.rs").rename(root / "docs/old.md")
            unusual = "docs/space and\nnewline.md"
            (root / unusual).write_text("text\n")
            # Exceed hosted path-filter limits without relying on their ordering.
            for index in range(3100):
                (root / "docs" / f"page-{index}.md").touch()
            git("add", ".")
            git("commit", "--quiet", "-m", "rename and docs")
            head = git("rev-parse", "HEAD")
            git("switch", "--quiet", "main")
            (root / "base-only.txt").write_text("main advanced\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "main update")
            original = Path.cwd()
            try:
                os.chdir(directory)
                paths = scope.changed_paths("main", head, pull_request=True)
                self.assertIn("crates/old.rs", paths)
                self.assertIn("docs/old.md", paths)
                self.assertIn(unusual, paths)
                self.assertNotIn("base-only.txt", paths)
                self.assertEqual(len(paths), 3103)
                self.assertIn("rust", selected(*paths))
                self.assertIn("base-only.txt", scope.changed_paths(base, "main", pull_request=False))
                event = root / "event.json"
                event.write_text(json.dumps({"pull_request": {
                    "base": {"sha": git("rev-parse", "main")}, "head": {"sha": head},
                }}))
                output, summary = root / "outputs.txt", root / "summary.md"
                subprocess.run([sys.executable, str(ROOT / "scripts/ci-scope.py")], check=True,
                               stdout=subprocess.PIPE, env={**os.environ,
                                   "GITHUB_EVENT_NAME": "pull_request", "GITHUB_EVENT_PATH": str(event),
                                   "GITHUB_OUTPUT": str(output), "GITHUB_STEP_SUMMARY": str(summary),
                               })
                decisions = dict(line.split("=") for line in output.read_text().splitlines())
                self.assertEqual(decisions["rust"], "true")
                self.assertEqual(decisions["packages"], "false")
                self.assertEqual(decisions["docs"], "true")
                self.assertIn("packages: not affected", summary.read_text())
                missing = subprocess.run([
                    sys.executable, str(ROOT / "scripts/ci-scope.py"),
                    "--base", "missing-ref", "--head", head,
                ], capture_output=True, text=True)
                self.assertNotEqual(missing.returncode, 0)
                self.assertIn("missing-ref", missing.stderr)
            finally:
                os.chdir(original)


if __name__ == "__main__":
    unittest.main()
