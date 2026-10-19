"""Local planning includes dirty source and reads its current configuration."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from test_scope import ROOT


class WorkingTreeScopeTests(unittest.TestCase):
    def test_staged_unstaged_untracked_and_ignored_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

            git("init", "-q", "--initial-branch=main")
            git("config", "user.name", "Scope fixture")
            git("config", "user.email", "scope@example.invalid")
            (root / ".gitignore").write_text("/target/\n")
            (root / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
            (root / "README.md").write_text("original\n")
            git("add", ".")
            git("-c", "commit.gpgsign=false", "commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            (root / "README.md").write_text("staged change\n")
            git("add", "README.md")
            (root / "Cargo.toml").write_text(
                "[workspace]\nmembers = []\n[workspace.metadata.typos.default.extend-words]\n"
                'ratatui = "ratatui"\n'
            )
            (root / "docs").mkdir()
            unusual = "docs/new and\ninteresting.md"
            (root / unusual).write_text("new document\n")
            (root / "target").mkdir()
            (root / "target/ignored.rs").write_text("ignored output\n")
            result = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "scripts/ci-scope.py"),
                    "--base",
                    base,
                    "--working-tree",
                    "--explain",
                ],
                cwd=root,
                check=True,
                capture_output=True,
                text=True,
                env={
                    key: value
                    for key, value in os.environ.items()
                    if key not in {"GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY"}
                },
            )
            lanes = json.loads(result.stdout)
            self.assertEqual(
                {lane for lane, run in lanes.items() if run}, {"docs", "maintainability"}
            )
            reasons = json.loads(result.stderr)
            self.assertIn(unusual, reasons["docs"]["inputs"])
            self.assertIn("Cargo.toml", reasons["maintainability"]["inputs"])
            self.assertNotIn("ignored.rs", result.stderr)
            # A real Cargo input in the same dirty file must select its consumers.
            (root / "Cargo.toml").write_text('[workspace]\nmembers = ["crates/new"]\n')
            changed = subprocess.check_output(
                [
                    sys.executable,
                    str(ROOT / "scripts/ci-scope.py"),
                    "--base",
                    base,
                    "--working-tree",
                ],
                cwd=root,
                text=True,
            )
            self.assertTrue(json.loads(changed)["rust"])
