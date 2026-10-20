"""Configuration edits select their consumers without building unrelated products."""

import os
import re
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch

from ci_scope_inputs import config_change_lanes
from test_scope import ROOT, scope, selected


def routed(before_path, after_path, before, after):
    with patch("ci_scope_inputs.file_at", side_effect=[before.encode(), after.encode()]):
        return config_change_lanes(before_path, after_path, "base", "head")


class ConfigInputTests(unittest.TestCase):
    def test_real_git_config_moves_and_spelling_metadata_use_only_tooling_checks(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def git(*args):
                return subprocess.check_output(["git", "-C", temporary, *args], text=True).strip()

            git("init", "--quiet", "--initial-branch=main")
            git("config", "user.name", "Config fixtures")
            git("config", "user.email", "config@example.invalid")
            git("config", "commit.gpgsign", "false")
            (root / ".mise.toml").write_text('[tools]\nrust = "1.96.0"\n')
            (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
            (root / "_typos.toml").write_text('[default.extend-words]\nratatui = "ratatui"\n')
            (root / "engines.just").write_text(
                'set shell := ["bash"]\n\nworkflow:\n    cargo test --locked --workspace\n'
            )
            git("add", ".")
            git("commit", "--quiet", "-m", "native root configuration")
            base = git("rev-parse", "HEAD")
            (root / ".config/mise").mkdir(parents=True)
            (root / ".config/just").mkdir()
            (root / ".mise.toml").rename(root / ".config/mise/config.toml")
            engine = (root / "engines.just").read_text()
            (root / "engines.just").unlink()
            (root / ".config/just/engines.just").write_text(
                "set working-directory := '../..'\n" + engine
            )
            (root / "_typos.toml").unlink()
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = []\n[workspace.metadata.typos.default.extend-words]\nratatui = "ratatui"\n'
            )
            git("add", ".")
            git("commit", "--quiet", "-m", "group native configuration")
            head = git("rev-parse", "HEAD")
            original = Path.cwd()
            try:
                os.chdir(root)
                changes = scope.changed_files(base, head, pull_request=True)
                self.assertIn(scope.Change(".mise.toml", ".config/mise/config.toml"), changes)
                lanes = scope.select_lanes(changes, base=base, head=head)
                self.assertEqual({lane for lane, run in lanes.items() if run}, {"maintainability"})
            finally:
                os.chdir(original)

    def test_mise_relocations_compare_settings_and_tool_upgrades_keep_consumers(self):
        for old_path, new_path, text in (
            (".mise.toml", ".config/mise/config.toml", '[tools]\nrust = "1.96.0"\n'),
            (
                "mise.packaging.toml",
                ".config/mise/config.packaging.toml",
                '[tools]\npackager = "1"\n',
            ),
            ("mise.lsp.toml", ".config/mise/config.lsp.toml", '[tools]\nuv = "1"\n'),
        ):
            with self.subTest(old_path=old_path):
                self.assertEqual(
                    routed(old_path, new_path, text, "# moved\n" + text), {"maintainability"}
                )
        self.assertEqual(
            routed(
                ".mise.toml",
                ".config/mise/config.toml",
                '[tools]\nrust = "1"',
                '[tools]\nrust = "2"',
            ),
            scope.RUST_BUILD,
        )
        self.assertIn(
            "lsp-sessions",
            routed(
                "mise.lsp.toml",
                ".config/mise/config.lsp.toml",
                '[tools]\nuv = "1"',
                '[tools]\nuv = "2"',
            ),
        )

    def test_workflow_tool_updates_do_not_select_product_builds(self):
        for tool in ("aqua:rhysd/actionlint", "aqua:zizmorcore/zizmor"):
            before = f'[tools]\n"{tool}" = "1"\n'
            after = before.replace('"1"', '"2"')
            self.assertEqual(
                routed(".config/mise/config.toml", ".config/mise/config.toml", before, after),
                {"maintainability"},
            )
        self.assertEqual(selected(".github/renovate.json"), {"maintainability"})

    def test_command_moves_preserve_effective_working_directory(self):
        before = 'set shell := ["bash"]\n\nworkflow:\n    cargo test\n'
        self.assertEqual(
            routed(
                "engines.just",
                ".config/just/engines.just",
                before,
                "set working-directory := '../..'\n" + before,
            ),
            {"maintainability"},
        )
        for preamble in ("set working-directory := '..'\n", "set working-directory := dynamic()\n"):
            self.assertIn(
                "hosts",
                routed("engines.just", ".config/just/engines.just", before, preamble + before),
            )

    def test_component_dependency_recipe_does_not_run_writer_tests(self):
        old = "supply-chain:\n    cargo deny --config deny.toml check\n"
        self.assertEqual(
            routed(
                "apps/writer/justfile",
                "apps/writer/justfile",
                old,
                old.replace(" --config deny.toml", ""),
            ),
            {"dependencies", "maintainability"},
        )
        self.assertIn(
            "writer",
            routed(
                "apps/writer/justfile",
                "apps/writer/justfile",
                "check:\n    old\n",
                "check:\n    new\n",
            ),
        )
        self.assertEqual(
            routed(
                "apps/writer/justfile",
                "apps/writer/justfile",
                "packaging:\n    old\n",
                "packaging *args:\n    new\n",
            ),
            {"package-config", "maintainability"},
        )

    def test_local_aggregate_changes_do_not_hide_changes_to_ci_capabilities(self):
        old = "_verify:\n    old\n\ncore-check:\n    old\n"
        new = old.replace("old", "new")
        self.assertEqual(routed("justfile", "justfile", old, new), {"rust", "maintainability"})

    def test_only_spelling_metadata_is_excluded_from_cargo_build_inputs(self):
        old = '[workspace]\nmembers = ["crates/core"]\n[workspace.metadata.recite]\ncritical_mutation_files = ["critical.rs"]\n'
        typos = '[workspace.metadata.typos.default.extend-words]\nratatui = "ratatui"\n'
        self.assertEqual(routed("Cargo.toml", "Cargo.toml", old, old + typos), {"maintainability"})
        for new in (
            old.replace("critical.rs", "other.rs"),
            old + '[workspace.dependencies]\nserde = "1"\n',
        ):
            self.assertEqual(routed("Cargo.toml", "Cargo.toml", old, new), scope.RUST_BUILD)
        self.assertIsNone(config_change_lanes("one/Cargo.toml", "two/Cargo.toml", "base", "head"))

    def test_only_local_tool_config_ignore_rules_avoid_mutation_tests(self):
        old = "/target/\n"
        self.assertEqual(
            routed(".gitignore", ".gitignore", old, old + "/.config/mise/config.*.local.toml\n"),
            {"maintainability"},
        )
        self.assertEqual(routed(".gitignore", ".gitignore", old, old + "crates/\n"), {"rust"})
        self.assertEqual(
            routed(".gitignore", ".gitignore", "*.rs\n!keep.rs\n", "!keep.rs\n*.rs\n"), {"rust"}
        )
        for before, after in (
            ("source/\n", " source/\n"),
            ("file\\ \n", "file\\\n"),
            ("# comment\n", " # pattern\n"),
            (".mise.local.toml\n", " .mise.local.toml\n"),
        ):
            self.assertEqual(routed(".gitignore", ".gitignore", before, after), {"rust"})

    def test_large_source_diff_does_not_spawn_git_reads_for_every_file(self):
        changes = [
            scope.Change(f"docs/page-{index}.md", f"docs/page-{index}.md") for index in range(3100)
        ]
        changes.append(scope.Change("Cargo.toml", "Cargo.toml"))
        reasons = {}
        with patch("ci_scope_inputs.file_at", return_value=b"[workspace]\nmembers = []\n") as read:
            lanes = scope.select_lanes(changes, base="base", head="head", reasons=reasons)
        self.assertEqual(read.call_count, 2)
        self.assertEqual({lane for lane, run in lanes.items() if run}, {"docs", "maintainability"})
        self.assertEqual(reasons["docs"]["count"], 3100)
        self.assertEqual(len(reasons["docs"]["inputs"]), 8)

    def test_native_builds_are_reserved_for_complete_events(self):
        for path in (
            "nix/packages.nix",
            "dist-workspace.toml",
            "apps/writer/packaging/common.json",
        ):
            with self.subTest(path=path):
                lanes = selected(path)
                self.assertIn("package-config", lanes)
                self.assertFalse(scope.DISTRIBUTION & lanes)
        for event in ("schedule", "workflow_dispatch"):
            self.assertTrue(all(scope.event_scope(event, {})[lane] for lane in scope.DISTRIBUTION))

    def test_quality_plan_is_independent_of_build_dependency_and_performance_work(self):
        def plan(recipe):
            return subprocess.check_output(
                ["just", "--dry-run", recipe], cwd=ROOT, text=True, stderr=subprocess.STDOUT
            )

        quality = plan("_check-quality")
        complete = plan("_verify")
        for operation in ("supply-chain", "unused-deps", "perf check", "perf setup"):
            self.assertNotIn(operation, quality)
            self.assertIn(operation, complete)
        for operation in (
            "_check-quality",
            "check-git-policy.sh",
            "tests/git-policy/check-integration.sh",
            "tests/trusted-policy/check.sh",
            "tests/editor-parity/check.sh",
            "check-vscode.sh",
            "check-helix.sh",
            "tests/editor-hosts/helix/check.sh",
            "check-project-gates.sh",
            "check-docs.sh",
            "perf smoke",
            "just coverage",
            "writer coverage",
            "mutants",
        ):
            self.assertIn(operation, complete)
        lint = subprocess.check_output(
            ["just", "--dry-run", "quality", "lint"], cwd=ROOT, text=True, stderr=subprocess.STDOUT
        )
        self.assertNotIn("web check", lint)
        self.assertNotIn("cargo", lint)
        packaging = subprocess.check_output(
            ["just", "--dry-run", "writer", "packaging"],
            cwd=ROOT,
            text=True,
            stderr=subprocess.STDOUT,
        )
        self.assertIn("--check-config", packaging)
        self.assertNotIn("cargo build", packaging)
        self.assertNotIn("package-writer-flatpak.sh", packaging)

    def test_quality_profile_tools_are_installed_before_the_action_saves_its_cache(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        job = workflow.split("\n  maintainability:\n", 1)[1].split("\n  packages:\n", 1)[0]
        install = re.search(r"install_args: >-\n((?:            [^\n]+\n)+)", job)
        self.assertIsNotNone(install)
        installed = set(install[1].split())
        profile = tomllib.loads((ROOT / ".config/mise/config.quality.toml").read_text())
        self.assertTrue(set(profile["tools"]) <= installed)
        self.assertFalse(
            {"cargo:cargo-deny", "cargo:cargo-machete", "cargo:cargo-mutants"} & installed
        )

    def test_native_quality_setup_leaves_unrelated_missing_tools_uninstalled(self):
        tools = tomllib.loads((ROOT / ".config/mise/config.toml").read_text())["tools"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            configs = root / ".config/mise"
            configs.mkdir(parents=True)
            (configs / "config.toml").write_text(
                '[tools]\nnode = "0.0.0"\n'
                '[tasks.quality-powershell-setup]\nrun = "echo scoped-profile-only"\n'
            )
            (configs / "config.quality.toml").write_text(f"[tools]\njust = {tools['just']!r}\n")
            scripts = root / "scripts"
            scripts.mkdir()
            script = scripts / "setup-quality.py"
            script.write_bytes((ROOT / "scripts/setup-quality.py").read_bytes())
            result = subprocess.run(
                [sys.executable, str(script)],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                env={
                    **os.environ,
                    "MISE_TRUSTED_CONFIG_PATHS": temporary,
                    "MISE_OFFLINE": "true",
                    "MISE_AUTO_INSTALL": "true",
                    "MISE_EXEC_AUTO_INSTALL": "false",
                    "MISE_TASK_RUN_AUTO_INSTALL": "true",
                },
                timeout=30,
            )
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("scoped-profile-only", result.stdout)

    def test_package_plan_gate_evaluates_all_platforms_without_native_builds(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        job = workflow.split("\n  package-config:\n", 1)[1].split("\n  maintainability:\n", 1)[0]
        self.assertIn("just writer packaging --builder", job)
        self.assertIn("desktop-file-validate", job)
        self.assertIn("appstreamcli validate --no-net", job)
        self.assertIn("nix flake check --no-build --all-systems", job)
        self.assertNotIn("nix build", job)

    def test_shared_config_changes_select_the_affected_surface(self):
        cases = (
            (
                ".config/mise/config.toml",
                b'[tools]\nnode = "24.18.0"\nbun = "1.4.2"\n',
                b'[tools]\nnode = "24.18.0"\nbun = "1.4.3"\n',
                scope.JS,
            ),
            (
                ".config/mise/config.toml",
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
            "coverage": {"rust", "maintainability"},
            "mutants": {"rust", "maintainability"},
            "host-check": {"hosts", "maintainability"},
            "editor-native-check": {"editor-native", "maintainability"},
            "check": scope.JUST,
            "_verify": scope.JUST,
        }.items():
            old = f'set shell := ["bash"]\n\nhelp:\n    same\n\n{recipe}:\n    old\n'.encode()
            with (
                self.subTest(recipe=recipe),
                patch("ci_scope_config.file_at", side_effect=[old, old.replace(b"old", b"new")]),
            ):
                self.assertEqual(scope.shared_config_lanes("justfile", "base", "head"), expected)
        for tool, before, after, expected in (
            ("node", "22", "24", scope.JS),
            ("rust", "1.95", "1.96", scope.RUST_BUILD),
            ("cargo:cargo-llvm-cov", "1", "2", scope.RUST_BUILD),
            ("cargo:cargo-mutants", "1", "2", scope.RUST_BUILD),
        ):
            old = f'[tools]\n"{tool}" = "{before}"\n'.encode()
            new = f'[tools]\n"{tool}" = "{after}"\n'.encode()
            with (
                self.subTest(tool=tool),
                patch("ci_scope_config.file_at", side_effect=[old, new]),
            ):
                self.assertEqual(
                    scope.shared_config_lanes(".config/mise/config.toml", "base", "head"), expected
                )
        workflow = ".github/workflows/ci.yml"
        for lane in scope.LANES - {"docs"}:
            old = f"name: CI\n\njobs:\n  {lane}:\n    old\n".encode()
            with (
                self.subTest(lane=lane),
                patch("ci_scope_config.file_at", side_effect=[old, old.replace(b"old", b"new")]),
            ):
                self.assertEqual(scope.shared_config_lanes(workflow, "base", "head"), {lane})
