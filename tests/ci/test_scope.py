"""Coverage selection contracts, including real Git rename and merge-base diffs."""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("ci_scope", ROOT / "scripts/ci-scope.py")
scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scope)


def selected(*paths):
    return {lane for lane, run in scope.select_lanes(paths).items() if run}


class ScopeTests(unittest.TestCase):
    def test_maintainability_exceptions_select_policy_and_docs(self):
        self.assertEqual(
            selected("scripts/maintainability/exceptions.toml"), {"docs", "maintainability"}
        )

    def test_core_and_dependencies_keep_contracts_without_distribution(self):
        for crate in ("runtime", "playground"):
            self.assertEqual(
                selected(f"crates/recite-{crate}/src/lib.rs"), scope.RUST | {"docs", "site"}
            )
        for path in ("Cargo.lock", "apps/writer/Cargo.lock", "crates/recite-core/src/lib.rs"):
            with self.subTest(path=path):
                lanes = selected(path)
                self.assertTrue(scope.RUST <= lanes)
                self.assertFalse(scope.DISTRIBUTION & lanes)

    def test_lsp_tooling_package_and_environment_select_their_consumers(self):
        for path in (
            "scripts/lsp.py",
            "scripts/lsp_tools/client.py",
            "perf.just",
            "mise.lsp.toml",
            "pyproject.toml",
            "uv.lock",
        ):
            self.assertEqual(selected(path), {"benchmark-smoke", "lsp-sessions", "maintainability"})

    def test_lsp_measurement_changes_select_benchmark_lane(self):
        for path in (
            "scripts/check-lsp-performance.py",
            "scripts/check-lsp-performance.sh",
            "scripts/measure-lsp-session.py",
            "scripts/lsp_regression.py",
            "scripts/lsp-performance-policy.json",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"benchmark-smoke", "maintainability"})

    def test_session_matrix_is_scoped_and_cannot_disable_pr_coverage(self):
        for path in (
            "scripts/measure-lsp-endurance.py",
            "scripts/lsp_session_health.py",
            "scripts/check-lsp-session-faults.py",
            "scripts/lsp-session-requirements.txt",
            "scripts/measure-lsp-channel-handoff.py",
            "scripts/summarize-lsp-server-comparison.py",
            "scripts/measure-lsp-stopped-responses.py",
            "scripts/measure-lsp-driver-accounting.py",
            ".github/workflows/lsp-sessions.yml",
        ):
            self.assertEqual(selected(path), {"lsp-sessions", "maintainability"})
        for path in ("crates/recite-lsp/src/server.rs", "crates/recite-compiler/src/lib.rs"):
            self.assertIn("lsp-sessions", selected(path))
        self.assertEqual(
            {
                lane
                for lane, value in scope.event_scope(
                    "workflow_dispatch", {"inputs": {"lsp_sessions_only": "true"}}
                ).items()
                if value
            },
            {"lsp-sessions"},
        )
        self.assertTrue(
            all(scope.event_scope("schedule", {"inputs": {"lsp_sessions_only": "true"}}).values())
        )
        with patch.object(scope, "changed_paths", return_value=["crates/recite-lsp/src/server.rs"]):
            selection = scope.event_scope(
                "pull_request",
                {
                    "inputs": {"lsp_sessions_only": "true"},
                    "pull_request": {"base": {"sha": "base"}, "head": {"sha": "head"}},
                },
            )
        self.assertTrue(selection["rust"] and selection["lsp-sessions"])

    def test_writer_keeps_ui_and_accessibility_without_packaging(self):
        for path in (
            "apps/writer/crates/freya/src/app.rs",
            "scripts/check-writer-native-accessibility.py",
            "tools/ast-grep/rules/rust-writer-palette.yml",
            "tools/ast-grep/tests/rust-writer-palette-test.yml",
        ):
            self.assertEqual(selected(path), {"writer", "maintainability"})

    def test_editor_and_schema_consumers(self):
        self.assertIn("editor", selected("crates/recite-lsp/src/lib.rs"))
        self.assertEqual(
            selected("editors/vscode/src/extension.ts"),
            {"editor", "maintainability", "lsp-sessions"},
        )
        self.assertEqual(
            selected("tests/editor-hosts/vscode/latency-probe.cjs"),
            {"editor", "maintainability", "lsp-sessions"},
        )
        self.assertTrue({"docs", "rust", "editor"} <= selected("schemas/manifest.json"))
        self.assertTrue({"docs", "rust", "editor"} <= selected("fixtures/schema/valid/test.json"))
        self.assertIn("rust", selected("fixtures/recite/markdown-input.md"))

    def test_engine_companions_select_the_rust_adapter_gate(self):
        for path in (
            "crates/recite-adapter/src/lib.rs",
            "crates/recite-bevy/src/lib.rs",
            "crates/recite-godot/src/lib.rs",
        ):
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

    def test_packaging_owners_select_only_their_distribution_family(self):
        for path, lane in (
            ("flake.nix", "nix-packages"),
            ("flake.lock", "nix-packages"),
            ("nix/packages.nix", "nix-packages"),
            ("apps/writer/packaging/flatpak/manifest.json", "flatpak-packages"),
            ("scripts/package-writer-flatpak.sh", "flatpak-packages"),
            ("scripts/check-writer-flatpak.py", "flatpak-packages"),
            ("tests/writer-flatpak/check.py", "flatpak-packages"),
            ("apps/writer/packaging/icons/recite-writer.svg", "flatpak-packages"),
            ("scripts/package-writer.py", "packages"),
            ("mise.packaging.toml", "packages"),
            ("scripts/check-writer-package-windows.ps1", "packages"),
            ("tests/writer-packaging/check.py", "packages"),
            ("apps/writer/packaging/common.json", "packages"),
            ("apps/writer/packaging/icons/recite-writer.ico", "packages"),
            ("assets/identity/recite.png", "packages"),
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {lane, "maintainability", "docs"})
        for path in (".mise.toml", ".github/workflows/ci.yml"):
            self.assertEqual(selected(path), scope.LANES)

    def test_release_owners_validate_plans_and_shared_rust_workflow_validates_callers(self):
        for path, builds_archives in {
            "dist-workspace.toml": True,
            "mise.release.toml": True,
            "release.just": True,
            ".github/workflows/cli-packages.yml": True,
            "release.toml": False,
            ".github/workflows/release.yml": False,
            ".github/workflows/publish-release.yml": False,
            "tools/recite-release/Cargo.toml": False,
            "tools/recite-release/src/main.rs": False,
        }.items():
            expected = {"rust", "release-plan", "maintainability", "docs"}
            if builds_archives:
                expected.add("cli-packages")
            with self.subTest(path=path):
                self.assertEqual(selected(path), expected)
        self.assertEqual(
            selected(".github/workflows/rust-checks.yml"),
            {"rust", "hosts", "writer", "editor-native", "maintainability"},
        )

    def test_ci_policy_changes_use_unconditional_contract_checks(self):
        for path in (
            "scripts/ci-scope.py",
            "scripts/ci_scope_config.py",
            "scripts/check-ci-results.py",
            "tests/ci/test_scope.py",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"maintainability"})

    def test_shared_distribution_inputs_and_wordmarks_select_all_consumers(self):
        for path, families in {
            ".github/workflows/writer-packages.yml": scope.WRITER_DISTRIBUTION,
            "LICENSE-MIT": scope.DISTRIBUTION,
        }.items():
            self.assertEqual(selected(path), families | {"maintainability", "docs"})
        self.assertEqual(
            selected("apps/writer/packaging/icons/recite-writer.png"),
            scope.WRITER_DISTRIBUTION | {"writer", "maintainability", "docs"},
        )
        for path in (
            "assets/identity/recite-wordmark.svg",
            "assets/identity/recite-wordmark-reversed.svg",
        ):
            with self.subTest(path=path):
                self.assertEqual(
                    selected(path),
                    scope.WRITER_DISTRIBUTION | {"writer", "maintainability", "docs", "site"},
                )

    def test_unknown_inputs_are_conservative_and_changes_union(self):
        self.assertEqual(selected("new-build-system/config.json"), scope.LANES)
        self.assertEqual(
            selected("docs/recite-production-spec.md", "crates/recite-cli/src/lib.rs"),
            {"docs"} | scope.RUST,
        )
        self.assertEqual(selected(), {"maintainability"})

    def test_shared_config_changes_select_the_affected_surface(self):
        cases = (
            (
                ".mise.toml",
                b'[tools]\nrust = "1.96"\nnode = "22"\n',
                b'[tools]\nrust = "1.96"\nnode = "24"\n',
                scope.JS,
            ),
            (
                ".mise.toml",
                b'[tools]\nrust = "1.95"\n',
                b'[tools]\nrust = "1.96"\n',
                scope.RUST_BUILD,
            ),
            (
                ".mise.toml",
                b'[tools]\nnode = "22"\n',
                b"[settings]\nexperimental = true\n",
                scope.LANES,
            ),
            (
                "justfile",
                b'set shell := ["bash"]\n\nfmt:\n    cargo fmt\n',
                b'set shell := ["zsh"]\n\nfmt:\n    cargo fmt\n',
                scope.JUST_QUALITY,
            ),
            (
                ".github/workflows/ci.yml",
                b"name: CI\n\njobs:\n  docs:\n    old\n  rust:\n    same\n",
                b"name: CI\n\njobs:\n  docs:\n    new\n  rust:\n    same\n",
                {"docs", "site"},
            ),
            (
                ".github/workflows/ci.yml",
                b"name: CI\n\njobs:\n  changes:\n    outputs:\n      rust: ${{ steps.scope.outputs.rust }}\n"
                b"  docs:\n    same\n  required-check:\n    needs:\n      - changes\n      - git-policy\n      - rust\n",
                b"name: CI\n\njobs:\n  changes:\n    outputs:\n      rust: ${{ steps.scope.outputs.rust }}\n"
                b"      site: ${{ steps.scope.outputs.site }}\n"
                b"  docs:\n    same\n  site:\n    new\n  required-check:\n    needs:\n      - changes\n      - git-policy\n      - rust\n      - site\n",
                {"site"},
            ),
            (
                ".github/workflows/ci.yml",
                b"name: CI\n\njobs:\n  changes:\n    steps:\n      - run: python3 scripts/ci-scope.py\n",
                b"name: CI\n\njobs:\n  changes:\n    steps:\n      - run: echo skip checks\n",
                scope.LANES,
            ),
        )
        for path, old, new, expected in cases:
            with self.subTest(path=path, new=new):
                with patch("ci_scope_config.file_at", side_effect=[old, new]):
                    self.assertEqual(scope.shared_config_lanes(path, "base", "head"), expected)
        for recipe, expected in {
            "fmt": scope.JUST,
            "_check-quality": scope.JUST,
            "clippy": {"rust", "maintainability"},
            "_clippy-rust": {"rust", "maintainability"},
            "core-check": {"rust", "maintainability"},
            "host-check": {"hosts", "maintainability"},
            "editor-native-check": {"editor-native", "maintainability"},
            "check": scope.JUST_QUALITY,
            "_verify": scope.JUST_QUALITY,
        }.items():
            old = f'set shell := ["bash"]\n\nhelp:\n    same\n\n{recipe}:\n    old\n'.encode()
            with (
                self.subTest(recipe=recipe),
                patch("ci_scope_config.file_at", side_effect=[old, old.replace(b"old", b"new")]),
            ):
                self.assertEqual(scope.shared_config_lanes("justfile", "base", "head"), expected)
        workflow = ".github/workflows/ci.yml"
        for lane in scope.LANES - {"docs"}:
            old = f"name: CI\n\njobs:\n  {lane}:\n    old\n".encode()
            with (
                self.subTest(lane=lane),
                patch("ci_scope_config.file_at", side_effect=[old, old.replace(b"old", b"new")]),
            ):
                self.assertEqual(scope.shared_config_lanes(workflow, "base", "head"), {lane})

    def test_full_events_and_initial_push(self):
        for event in ("schedule", "workflow_dispatch"):
            self.assertTrue(all(scope.event_scope(event, {}).values()))
        self.assertTrue(
            all(scope.event_scope("push", {"before": "0" * 40, "after": "head"}).values())
        )
        with self.assertRaises(ValueError):
            scope.event_scope("pull_request_target", {})
        with self.assertRaises(KeyError):
            scope.event_scope("pull_request", {})

    def test_event_uses_pr_heads_and_direct_push_range(self):
        with patch.object(scope, "changed_paths", return_value=["README.md"]) as diff:
            scope.event_scope(
                "pull_request",
                {
                    "pull_request": {
                        "base": {"sha": "base"},
                        "head": {"sha": "head"},
                    }
                },
            )
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
                self.assertIn(
                    "base-only.txt", scope.changed_paths(base, "main", pull_request=False)
                )
                event = root / "event.json"
                event.write_text(
                    json.dumps(
                        {
                            "pull_request": {
                                "base": {"sha": git("rev-parse", "main")},
                                "head": {"sha": head},
                            }
                        }
                    )
                )
                output, summary = root / "outputs.txt", root / "summary.md"
                subprocess.run(
                    [sys.executable, str(ROOT / "scripts/ci-scope.py")],
                    check=True,
                    stdout=subprocess.PIPE,
                    env={
                        **os.environ,
                        "GITHUB_EVENT_NAME": "pull_request",
                        "GITHUB_EVENT_PATH": str(event),
                        "GITHUB_OUTPUT": str(output),
                        "GITHUB_STEP_SUMMARY": str(summary),
                    },
                )
                decisions = dict(line.split("=") for line in output.read_text().splitlines())
                self.assertEqual(decisions["rust"], "true")
                self.assertEqual(decisions["packages"], "false")
                self.assertEqual(decisions["docs"], "true")
                self.assertIn("packages: not affected", summary.read_text())
                missing = subprocess.run(
                    [
                        sys.executable,
                        str(ROOT / "scripts/ci-scope.py"),
                        "--base",
                        "missing-ref",
                        "--head",
                        head,
                    ],
                    capture_output=True,
                    text=True,
                )
                self.assertNotEqual(missing.returncode, 0)
                self.assertIn("missing-ref", missing.stderr)
            finally:
                os.chdir(original)


if __name__ == "__main__":
    unittest.main()
