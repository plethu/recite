#!/usr/bin/env python3
"""Focused regression tests for Zed transport evidence assertions."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from assert_lsp_log import is_unsupported_empty_code_action_result  # noqa: E402


class CodeActionEvidenceTests(unittest.TestCase):
    def test_only_exact_empty_list_counts_as_unsupported_result(self) -> None:
        self.assertTrue(is_unsupported_empty_code_action_result([]))
        for result in ([[]], [None], None, {}, False):
            with self.subTest(result=result):
                self.assertFalse(is_unsupported_empty_code_action_result(result))


if __name__ == "__main__":
    unittest.main()
