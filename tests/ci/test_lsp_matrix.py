"""Session orchestration must preserve alternating order and process isolation."""

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.lsp_tools.matrix import run


class MatrixTests(unittest.TestCase):
    def test_pairs_alternate_and_only_native_children_receive_trace_paths(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.dict("os.environ", {"RECITE_LSP_TRACE_DIR": "inherited-trace"}),
            patch("scripts.lsp_tools.matrix.subprocess.run") as launch,
        ):
            root = Path(directory)
            run(Path("candidate"), root, control=Path("control"), native_trace=True)
            observed = []
            for call in launch.call_args_list:
                args = call.args[0]
                report = Path(args[args.index("--output") + 1]).relative_to(root)
                observed.append(str(report))
                self.assertTrue(call.kwargs["check"])
                trace = call.kwargs["env"].get("RECITE_LSP_TRACE_DIR")
                if report.parts[0] == "native":
                    self.assertEqual(Path(trace), root / report.with_suffix(""))
                else:
                    self.assertIsNone(trace)
            self.assertEqual(
                observed,
                [
                    f"{side}{mode}-{repetition}.json"
                    for repetition, sides in (
                        (1, ("native/", "control/", "")),
                        (2, ("", "control/", "native/")),
                        (3, ("native/", "control/", "")),
                    )
                    for side in sides
                    for mode in ("fixed", "churn")
                ],
            )

    def test_failed_observation_stops_the_matrix(self):
        with patch(
            "scripts.lsp_tools.matrix.subprocess.run",
            side_effect=subprocess.CalledProcessError(1, "driver"),
        ) as launch:
            with self.assertRaises(subprocess.CalledProcessError):
                run(Path("candidate"), Path("reports"))
            self.assertEqual(launch.call_count, 1)
