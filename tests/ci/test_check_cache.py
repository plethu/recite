"""Exercise real Turborepo results across working copies and failed checks."""

import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("check_runner", ROOT / "scripts/run-checks.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ContextTests(unittest.TestCase):
    def metadata(self, version="1.50.1", installed=True):
        return {
            tool: [{"active": True, "installed": present, "version": resolved}]
            for tool, resolved, present in [
                ("typos", version, installed),
                ("bun", "1.4.3", True),
                ("github:vercel/turborepo", "2.11.9", True),
            ]
        }

    def test_resolved_version_changes_identity_without_changing_source(self):
        def context(version):
            with (
                patch.dict(os.environ, {}, clear=True),
                patch.object(runner, "tool_bound", return_value="/tools/bin/tool"),
                patch.object(
                    runner,
                    "output",
                    side_effect=[
                        json.dumps(self.metadata(version)),
                        "mise pinned",
                        "git pinned",
                    ],
                ) as calls,
            ):
                result = runner.contexts({"spelling"})
            # A fixed request probes tools once, never once per source file.
            self.assertEqual(calls.call_count, 3)
            self.assertEqual(calls.call_args_list[0].args, ("mise", "ls", "--json", "--current"))
            return result

        self.assertEqual(context("1.50.1"), context("1.50.1"))
        self.assertNotEqual(context("1.50.1"), context("1.49.0"))

    def test_missing_or_system_tools_cannot_create_reusable_success(self):
        for version, installed in [("system", True), ("1.50.1", False)]:
            with (
                self.subTest(version=version, installed=installed),
                patch.object(runner, "tool_bound", return_value="/tools/bin/tool"),
                patch.object(
                    runner,
                    "output",
                    side_effect=[
                        json.dumps(self.metadata(version, installed)),
                        "mise pinned",
                        "git pinned",
                    ],
                ),
                self.assertRaisesRegex(ValueError, "Install the pinned check tool.*typos"),
            ):
                runner.contexts({"spelling"})


@unittest.skipUnless(
    shutil.which("turbo") and shutil.which("typos"), "requires pinned Turborepo and typos"
)
class NativeCacheTests(unittest.TestCase):
    def test_dirty_worktree_restore_failure_and_portable_results(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "source"
            root.mkdir()
            (root / ".config/mise").mkdir(parents=True)
            (root / "scripts").mkdir()
            shutil.copy(ROOT / "bunfig.toml", root / "bunfig.toml")
            config = json.loads((ROOT / "turbo.json").read_text())
            config["tasks"]["//#check:late"] = {
                "dependsOn": ["//#check:spelling"],
                "cache": False,
            }
            (root / "turbo.json").write_text(json.dumps(config))
            shutil.copy(ROOT / "scripts/run-checks.py", root / "scripts/run-checks.py")
            (root / ".config/mise/config.quality.toml").write_text("[tools]\n")
            import tomllib

            tools = tomllib.loads((ROOT / ".config/mise/config.toml").read_text())["tools"]
            manager = json.loads((ROOT / "package.json").read_text())["packageManager"]
            (root / ".config/mise/config.toml").write_text(
                f'[tools]\ntypos = "{tools["typos"]}"\n'
                f'bun = "{manager.split("@")[1]}"\n'
                f'"github:vercel/turborepo" = {{ version = "2.11.9", rename_exe = "turbo" }}\n'
            )
            (root / "nested").mkdir()
            (root / "nested/package.json").write_text(
                json.dumps(
                    {
                        "name": "nested",
                        "private": True,
                        "scripts": {
                            "probe": "python3 -c \"import os; assert 'SHELLCHECK_OPTS' not in os.environ\""
                        },
                    }
                )
            )
            (root / "package.json").write_text(
                json.dumps(
                    {
                        "name": "cache-fixture",
                        "private": True,
                        "packageManager": manager,
                        "workspaces": ["nested"],
                        "scripts": {
                            "check:spelling": "python3 scripts/probe.py",
                            "check:late": "python3 -c 'raise SystemExit(9)'",
                        },
                    }
                )
            )
            subprocess.run(
                ["bun", "install", "--lockfile-only", "--ignore-scripts"],
                cwd=root,
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )
            (root / ".gitignore").write_text("/.turbo/\n/runs\n/node_modules/\n/target/\n.env\n")
            (root / ".env").write_text("SHELLCHECK_OPTS=--exclude=SC2154\n")
            (root / "nested/.env").write_text("SHELLCHECK_OPTS=--exclude=SC2154\n")
            (root / "justfile").write_text("default:\n    true\n")
            (root / "text.txt").write_text("hello world\n")
            (root / "scripts/probe.py").write_text(
                "from pathlib import Path\nimport os, subprocess\np=Path('runs')\n"
                "assert not any(name in os.environ for name in ['SHELLCHECK_OPTS', 'DPRINT_CONFIG_DISCOVERY', 'RIPGREP_CONFIG_PATH'])\n"
                "p.write_text((p.read_text() if p.exists() else '')+'run\\n')\n"
                "subprocess.run(['bun', 'run', '--no-env-file', '--cwd', 'nested', 'probe'], check=True)\n"
                "raise SystemExit(subprocess.call(['typos']))\n"
            )
            environment = {
                **os.environ,
                "MISE_TRUSTED_CONFIG_PATHS": temporary,
                "MISE_ENV": "",
                "TURBO_TELEMETRY_DISABLED": "1",
                "SHELLCHECK_OPTS": "--exclude=SC2154",
                "DPRINT_CONFIG_DISCOVERY": "global",
                "RIPGREP_CONFIG_PATH": "/absent/ambient-config",
            }

            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, env=environment, text=True)

            git("init", "-q", "--initial-branch=main")
            git("config", "user.name", "Cache fixture")
            git("config", "user.email", "cache@example.invalid")
            git("add", ".")
            git("-c", "commit.gpgsign=false", "commit", "-qm", "base")

            def run(directory, *, success=True):
                result = subprocess.run(
                    ["python3", "scripts/run-checks.py", "spelling"],
                    cwd=directory,
                    env=environment,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    timeout=30,
                )
                self.assertEqual(result.returncode == 0, success, result.stdout)
                path = directory / "runs"
                return len(path.read_text().splitlines()) if path.exists() else 0

            # A later failure must not discard an already successful dependency.
            with (
                patch.object(runner, "ROOT", root),
                patch.dict(os.environ, environment, clear=True),
            ):
                check_environment = {**environment, **runner.contexts({"spelling"})}
            failed = subprocess.run(
                [
                    "turbo",
                    "run",
                    "check:late",
                    "--filter=//",
                    "--cache=local:rw",
                    "--framework-inference=false",
                    "--env-mode=strict",
                ],
                cwd=root,
                env=check_environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
            )
            self.assertNotEqual(failed.returncode, 0, failed.stdout)
            self.assertEqual(run(root), 1)
            (root / "target").mkdir()
            (root / "target/ignored.txt").write_text("ignored output changes\n")
            self.assertEqual(run(root), 1)
            override = subprocess.run(
                [
                    "mise",
                    "exec",
                    "typos@system",
                    "--",
                    "python3",
                    "scripts/run-checks.py",
                    "spelling",
                ],
                cwd=root,
                env=environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
            )
            self.assertNotEqual(override.returncode, 0, override.stdout)
            self.assertIn("Install the pinned check tool", override.stdout)
            self.assertEqual(run(root), 1)
            worktree = Path(temporary) / "worktree"
            git("worktree", "add", "--detach", str(worktree), "HEAD")
            self.assertEqual(run(worktree), 0)  # A different directory reuses the same real task.
            (worktree / "text.txt").write_text("hello there\n")
            self.assertEqual(run(worktree), 1)
            (worktree / "text.txt").write_text("hello world\n")
            self.assertEqual(run(worktree), 1)
            (worktree / "nested/untracked.txt").write_text("t" + "eh\n")
            self.assertEqual(run(worktree, success=False), 2)
            self.assertEqual(run(worktree, success=False), 3)
            (worktree / "nested/untracked.txt").unlink()
            self.assertEqual(run(worktree), 3)
            # Independent checkout: transfer only portable CAS, just as Actions does.
            clone = Path(temporary) / "fresh"
            subprocess.run(["git", "clone", "-q", "--no-local", str(root), str(clone)], check=True)
            shutil.copytree(root / ".turbo/cache", clone / ".turbo/cache")
            self.assertEqual(run(clone), 0)
