"""Exercise the native release recipe's baseline policy without building Rust."""

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class ReleaseApiTests(unittest.TestCase):
    def test_explicit_baselines_initial_release_and_fixed_native_invocation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            recipes = root / ".config/just"
            recipes.mkdir(parents=True)
            shutil.copy(ROOT / ".config/just/release.just", recipes / "release.just")
            (root / "justfile").write_text("mod release '.config/just/release.just'\n")
            binaries = root / "bin"
            binaries.mkdir()
            mise = binaries / "mise"
            mise.write_text(
                "#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\n"
                "p=Path('calls.json')\ncalls=json.loads(p.read_text()) if p.exists() else []\n"
                "calls.append(sys.argv[1:])\np.write_text(json.dumps(calls))\n"
                "print('semver fixture diagnostics', file=sys.stderr)\n"
                "raise SystemExit(int(os.environ.get('SEMVER_EXIT', '0')))\n"
            )
            mise.chmod(0o755)
            environment = {**os.environ, "PATH": f"{binaries}{os.pathsep}{os.environ['PATH']}"}

            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

            git("init", "-q", "--initial-branch=main")
            git("config", "user.name", "Release fixture")
            git("config", "user.email", "release@example.invalid")
            git("config", "commit.gpgsign", "false")
            git("config", "tag.gpgsign", "false")
            git("add", ".")
            git("commit", "-qm", "baseline")
            baseline = git("rev-parse", "HEAD")
            git("commit", "--allow-empty", "-qm", "candidate")

            def run(baseline, success):
                result = subprocess.run(
                    ["just", "release", "api-check", baseline],
                    cwd=root,
                    env=environment,
                    capture_output=True,
                    text=True,
                    timeout=10,
                )
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
                return result.stdout

            self.assertIn("compatibility is unmeasured", run("initial", True))
            self.assertFalse((root / "calls.json").exists())
            self.assertIn(baseline, run(baseline, True))
            calls = json.loads((root / "calls.json").read_text())
            self.assertEqual(len(calls), 9)
            crates = set()
            archives = set()
            for args in calls:
                self.assertEqual(
                    args[:6], ["-E", "release", "exec", "--", "cargo", "semver-checks"]
                )
                self.assertIn("--default-features", args)
                self.assertEqual(args[args.index("--release-type") + 1], "minor")
                manifest = Path(args[args.index("--manifest-path") + 1])
                baseline_root = Path(args[args.index("--baseline-root") + 1])
                self.assertEqual(baseline_root.name, manifest.parent.name)
                crates.add(manifest.parent.name)
                archives.add(baseline_root.parents[1])
            self.assertIn("recite-compiler", crates)
            self.assertEqual(len(crates), 9)
            self.assertEqual(len(archives), 1)
            self.assertFalse(archives.pop().exists())
            (root / "calls.json").unlink()
            workflow = (ROOT / ".github/workflows/release.yml").read_text()
            step = re.search(
                r"      - name: Check and record Rust API compatibility\n(.*?)(?=      -)",
                workflow,
                re.S,
            ).group(1)
            self.assertIn("        shell: bash\n", step)
            pipeline = re.search(r"        run: (.+)", step).group(1)
            (root / "target").mkdir()
            result = subprocess.run(
                ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", pipeline],
                cwd=root,
                env={**environment, "API_BASELINE": baseline, "SEMVER_EXIT": "100"},
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            report = root / "target/candidate-api-compatibility.txt"
            self.assertIn("semver fixture diagnostics", report.read_text())
            self.assertEqual(len(json.loads((root / "calls.json").read_text())), 1)
            (root / "calls.json").unlink()
            run("HEAD", False)
            run("absent-revision", False)
            git("tag", "v0.1.0", baseline)
            run("initial", False)
            git("checkout", "-qb", "unrelated", baseline)
            git("commit", "--allow-empty", "-qm", "different history")
            unrelated = git("rev-parse", "HEAD")
            git("checkout", "-q", "main")
            run(unrelated, False)
            self.assertFalse((root / "calls.json").exists())
