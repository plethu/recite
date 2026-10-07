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
                     "apps/writer/packaging/README.md"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs"})

    def test_site_and_shared_javascript_do_not_select_native_builds(self):
        self.assertEqual(selected("docs-site/src/content/docs/index.mdx"), {"docs", "site"})
        self.assertEqual(selected("docs-site/src/content/docs/reference/index.md"),
                         {"docs", "site"})
        self.assertEqual(selected("docs-site/README.md"), {"docs"})
        self.assertEqual(selected("docs-site/justfile"), {"docs", "site"})
        for path in ("docs-site/astro.config.mjs", "docs-site/check-browser.sh"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "site", "maintainability"})
        for path in ("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.JS)
        self.assertEqual(selected("scripts/install-js-dependencies.sh"),
                         scope.JS | {"maintainability"})
        for path in ("scripts/check-docs.sh", "scripts/check-schema-manifest.mjs",
                     "scripts/check-site-links.py"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "site", "maintainability"})
        self.assertEqual(selected(".gitignore"), set())
        for path in ("apps/writer/justfile", "editors/justfile",
                     "editors/zed/justfile", "engines.just", "stress.just"):
            self.assertEqual(selected(path), scope.JUST)
        for path in ("scripts/check-project-gates.sh", "scripts/check-ffi-header.sh"):
            self.assertEqual(selected(path), {"rust", "maintainability"})
        self.assertEqual(selected("scripts/check-zed.sh"),
                         {"rust", "editor", "maintainability"})
        self.assertEqual(selected("justfile"), scope.JUST)
        self.assertNotIn("packages", selected("justfile"))
        self.assertNotIn("windows-publisher", selected("justfile"))
        self.assertEqual(selected("docs-site/src/content/docs/index.mdx", "justfile",
                                  "apps/writer/justfile"),
                         {"docs", "site", "maintainability"})
        self.assertNotIn("site", selected("crates/recite-runtime/src/lib.rs"))
        self.assertNotIn("docs", selected("crates/recite-runtime/src/lib.rs"))
        for path in ("Cargo.toml", "Cargo.lock", "crates/recite-core/Cargo.toml",
                     "apps/writer/Cargo.lock", ".cargo/config.toml"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.RUST_BUILD)

    def test_maintainability_exceptions_select_policy_and_docs(self):
        self.assertEqual(selected("scripts/maintainability/exceptions.toml"),
                         {"docs", "maintainability"})

    def test_core_changes_keep_windows_and_benchmarks(self):
        self.assertEqual(selected("crates/recite-runtime/src/lib.rs"), {
            "rust", "windows-publisher", "benchmark-smoke", "editor", "maintainability",
        })

    def test_lsp_tooling_package_and_environment_select_their_consumers(self):
        for path in ("scripts/lsp.py", "scripts/lsp_tools/client.py", "perf.just",
                     "mise.lsp.toml", "pyproject.toml", "uv.lock"):
            self.assertEqual(selected(path), {"benchmark-smoke", "lsp-sessions", "maintainability"})

    def test_lsp_measurement_changes_select_benchmark_lane(self):
        for path in ("scripts/check-lsp-performance.py", "scripts/check-lsp-performance.sh",
                     "scripts/measure-lsp-session.py", "scripts/lsp_regression.py",
                     "scripts/lsp-performance-policy.json"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"benchmark-smoke", "maintainability"})

    def test_session_matrix_is_scoped_and_cannot_disable_pr_coverage(self):
        for path in ("scripts/measure-lsp-endurance.py", "scripts/lsp_session_health.py",
                     "scripts/check-lsp-session-faults.py", "scripts/lsp-session-requirements.txt",
                     "scripts/measure-lsp-channel-handoff.py", "scripts/summarize-lsp-server-comparison.py",
                     "scripts/measure-lsp-stopped-responses.py", "scripts/measure-lsp-driver-accounting.py",
                     ".github/workflows/lsp-sessions.yml"):
            self.assertEqual(selected(path), {"lsp-sessions", "maintainability"})
        for path in ("crates/recite-lsp/src/server.rs", "crates/recite-compiler/src/lib.rs"):
            self.assertIn("lsp-sessions", selected(path))
        self.assertEqual({lane for lane, value in scope.event_scope(
            "workflow_dispatch", {"inputs": {"lsp_sessions_only": "true"}}).items() if value}, {"lsp-sessions"})
        self.assertTrue(all(scope.event_scope("schedule", {"inputs": {"lsp_sessions_only": "true"}}).values()))
        with patch.object(scope, "changed_paths", return_value=["crates/recite-lsp/src/server.rs"]):
            selection = scope.event_scope("pull_request", {
                "inputs": {"lsp_sessions_only": "true"},
                "pull_request": {"base": {"sha": "base"}, "head": {"sha": "head"}},
            })
        self.assertTrue(selection["rust"] and selection["lsp-sessions"])

    def test_writer_keeps_ui_and_accessibility_without_packaging(self):
        self.assertEqual(selected("apps/writer/crates/freya/src/app.rs"), {
            "rust", "maintainability",
        })
        self.assertIn("rust", selected("scripts/check-writer-native-accessibility.py"))

    def test_editor_and_schema_consumers(self):
        self.assertIn("editor", selected("crates/recite-lsp/src/lib.rs"))
        self.assertEqual(selected("editors/vscode/src/extension.ts"), {"editor", "maintainability", "lsp-sessions"})
        self.assertEqual(selected("tests/editor-hosts/vscode/latency-probe.cjs"),
                         {"editor", "maintainability", "lsp-sessions"})
        self.assertTrue({"docs", "rust", "editor"} <= selected("schemas/manifest.json"))
        self.assertTrue({"docs", "rust", "editor"} <= selected("fixtures/schema/valid/test.json"))
        self.assertIn("rust", selected("fixtures/recite/markdown-input.md"))

    def test_engine_companions_select_the_rust_adapter_gate(self):
        for path in ("crates/recite-adapter/src/lib.rs", "crates/recite-bevy/src/lib.rs",
                     "crates/recite-godot/src/lib.rs"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.RUST)
        for path in (
            "addons/com.recite.dialogue/plugin.cfg",
            "Packages/com.recite.dialogue/Runtime/ReciteDialogueService.cs",
            "examples/godot/basic-dialogue/project.godot",
            "tests/godot-host/run_tests.gd",
            "tests/unity-project/Packages/manifest.json",
            "scripts/check-godot-host.sh",
            "scripts/package-godot-addon.sh",
            "scripts/check-unity-adapter.sh",
            "scripts/check-bevy-package.sh",
            "scripts/unity/build-upm.sh",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.ENGINE)
        for path in (
            "addons/recite/README.md",
            "Packages/com.recite.dialogue/README.md",
            "examples/godot/basic-dialogue/README.md",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.ENGINE | {"docs"})

    def test_packaging_and_shared_build_inputs(self):
        for path in (
            "flake.nix", "apps/writer/packaging/flatpak/manifest.json",
            "scripts/package-writer.py", "assets/identity/recite.png",
            "apps/writer/Cargo.lock", "crates/recite-core/Cargo.toml",
        ):
            with self.subTest(path=path):
                self.assertIn("packages", selected(path))
        for path in (".mise.toml", ".github/workflows/ci.yml"):
            self.assertEqual(selected(path), scope.LANES)

    def test_ci_policy_changes_use_unconditional_contract_checks(self):
        for path in ("scripts/ci-scope.py", "scripts/check-ci-results.py",
                     "tests/ci/test_scope.py"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"maintainability"})

    def test_shared_wordmarks_select_site_browser_checks(self):
        for path in ("assets/identity/recite-wordmark.svg",
                     "assets/identity/recite-wordmark-reversed.svg"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"packages", "maintainability", "docs", "site"})
        self.assertEqual(selected("assets/identity/recite.png"),
                         {"packages", "maintainability", "docs"})

    def test_unknown_inputs_are_conservative_and_changes_union(self):
        self.assertEqual(selected("new-build-system/config.json"), scope.LANES)
        self.assertEqual(selected("docs/recite-production-spec.md", "crates/recite-cli/src/lib.rs"),
                         {"docs"} | scope.RUST)
        self.assertEqual(selected(), set())

    def test_shared_config_changes_select_the_affected_surface(self):
        cases = (
            (".mise.toml", b'[tools]\nrust = "1.96"\nnode = "22"\n',
             b'[tools]\nrust = "1.96"\nnode = "24"\n', scope.JS),
            (".mise.toml", b'[tools]\nrust = "1.95"\n',
             b'[tools]\nrust = "1.96"\n', scope.RUST_BUILD),
            (".mise.toml", b'[tools]\nnode = "22"\n',
             b'[settings]\nexperimental = true\n', scope.LANES),
            ("justfile", b'set shell := ["bash"]\n\nfmt:\n    cargo fmt\n',
             b'set shell := ["bash"]\n\nfmt:\n    cargo fmt --all\n', scope.JUST),
            ("justfile", b'set shell := ["bash"]\n\nclippy:\n    cargo clippy\n',
             b'set shell := ["bash"]\n\nclippy:\n    cargo clippy --all\n',
             {"rust", "maintainability"}),
            ("justfile", b'set shell := ["bash"]\n\ncheck:\n    scripts/verify.sh\n',
             b'set shell := ["bash"]\n\ncheck:\n    scripts/verify.sh --all\n',
             scope.JUST_QUALITY),
            ("justfile", b'set shell := ["bash"]\n\nfmt:\n    cargo fmt\n',
             b'set shell := ["zsh"]\n\nfmt:\n    cargo fmt\n',
             scope.JUST_QUALITY),
            (".github/workflows/ci.yml", b'name: CI\n\njobs:\n  docs:\n    old\n  rust:\n    same\n',
             b'name: CI\n\njobs:\n  docs:\n    new\n  rust:\n    same\n', {"docs", "site"}),
            (".github/workflows/ci.yml", b'name: CI\n\njobs:\n  rust:\n    old\n',
             b'name: CI\n\njobs:\n  rust:\n    new\n', {"rust"}),
            (".github/workflows/ci.yml",
             b'name: CI\n\njobs:\n  changes:\n    outputs:\n      rust: ${{ steps.scope.outputs.rust }}\n'
             b'  docs:\n    same\n  required-check:\n    needs:\n      - changes\n      - git-policy\n      - rust\n',
             b'name: CI\n\njobs:\n  changes:\n    outputs:\n      rust: ${{ steps.scope.outputs.rust }}\n'
             b'      site: ${{ steps.scope.outputs.site }}\n'
             b'  docs:\n    same\n  site:\n    new\n  required-check:\n    needs:\n      - changes\n      - git-policy\n      - rust\n      - site\n', {"site"}),
            (".github/workflows/ci.yml",
             b'name: CI\n\njobs:\n  changes:\n    steps:\n      - run: python3 scripts/ci-scope.py\n',
             b'name: CI\n\njobs:\n  changes:\n    steps:\n      - run: echo skip checks\n',
             scope.LANES),
        )
        for path, old, new, expected in cases:
            with self.subTest(path=path, new=new):
                with patch.object(scope, "file_at", side_effect=[old, new]):
                    self.assertEqual(scope.shared_config_lanes(path, "base", "head"), expected)

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
