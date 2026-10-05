"""Session decisions must distinguish bounded caches from sustained growth."""

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from lsp_session_health import assess


def rows():
    return [{"rss_bytes": 64 * 1024**2, "threads": 6, "handles": 5,
             "completion_ms": 5, "definition_ms": 1} for _ in range(35)]


class SessionHealthTests(unittest.TestCase):
    def test_flat_and_warming_caches_pass(self):
        sample = rows()
        for index, row in enumerate(sample):
            row["rss_bytes"] += (64 * 1024**2 if index >= 5 else index * 1024**2)
        self.assertEqual(assess(sample)["status"], "pass")

    def test_isolated_peak_and_cache_step_are_not_sustained_growth(self):
        sample = rows()
        for row in sample[18:]:
            row["rss_bytes"] *= 2
        sample[10]["rss_bytes"] *= 8
        self.assertEqual(assess(sample)["status"], "pass")

    def test_growth_and_slowdown_fail(self):
        for key, step in [("rss_bytes", 4 * 1024**2), ("threads", 1), ("handles", 2),
                          ("completion_ms", 2), ("definition_ms", 2)]:
            sample = rows()
            for index, row in enumerate(sample):
                row[key] += index * step
            with self.subTest(key=key):
                self.assertIn(key, assess(sample)["failures"])

    def test_short_missing_or_invalid_evidence_fails_closed(self):
        with self.assertRaises(ValueError):
            assess(rows()[:19])
        for value in [float("nan"), float("inf"), -1]:
            sample = rows()
            sample[-1]["rss_bytes"] = value
            with self.assertRaises(ValueError):
                assess(sample)
        sample = rows()
        del sample[-1]["handles"]
        with self.assertRaises(KeyError):
            assess(sample)
