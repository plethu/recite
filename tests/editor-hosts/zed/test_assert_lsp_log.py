#!/usr/bin/env python3
"""Focused regression tests for Zed transport evidence assertions."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from assert_lsp_log import has_canonical_missing_id_quick_fix  # noqa: E402


class CodeActionEvidenceTests(unittest.TestCase):
    def test_only_canonical_non_empty_quick_fix_counts_as_supported_result(self) -> None:
        self.assertTrue(
            has_canonical_missing_id_quick_fix(
                [{"title": "Insert missing stable ID", "kind": "quickfix", "edit": {"documentChanges": [{}]}}]
            )
        )
        for result in ([], [[]], [None], [{"title": "Insert missing stable ID"}], None, {}, False):
            with self.subTest(result=result):
                self.assertFalse(has_canonical_missing_id_quick_fix(result))


class HostFixtureIsolationTests(unittest.TestCase):
    def test_incomplete_fixture_is_excluded_from_project_stable_id_planning(self) -> None:
        script = (Path(__file__).parents[3] / "scripts/check-zed-host.sh").read_text(encoding="utf-8")
        self.assertIn('excludes = ["host-fixtures/**"]', script)
        self.assertIn('cp -- "$fixture" "$fixture_dir/fixture.recite"', script)


if __name__ == "__main__":
    unittest.main()
