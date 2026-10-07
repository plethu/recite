"""Session decisions must distinguish bounded caches from sustained growth."""

import unittest

from scripts.lsp_tools.health import assess, recovery_tail


def rows():
    return [
        {
            "rss_bytes": 64 * 1024**2,
            "threads": 6,
            "handles": 5,
            "completion_ms": 5,
            "definition_ms": 1,
        }
        for _ in range(35)
    ]


class SessionHealthTests(unittest.TestCase):
    def test_tail_budget_requires_repeated_slowdown(self):
        normal = [10.0] * 35
        slow_tail = [10.0] * 30 + [80.0] * 5
        self.assertEqual(recovery_tail([normal, slow_tail, normal], 50)["status"], "pass")
        result = recovery_tail([slow_tail, normal, slow_tail], 50)
        self.assertEqual(result["status"], "regression")
        self.assertEqual(result["p95_ms"], [80.0, 10.0, 80.0])
        isolated = normal[:-1] + [100.0]
        self.assertEqual(recovery_tail([isolated] * 3, 50)["status"], "pass")

    def test_tail_budget_rejects_missing_or_invalid_evidence(self):
        for samples in ([], [1.0] * 14, [float("nan")] * 35, [-1.0] * 35):
            with self.assertRaises(ValueError):
                recovery_tail([samples] * 3, 50)
        with self.assertRaises(ValueError):
            recovery_tail([[10.0] * 35] * 2, 50)
        with self.assertRaises(ValueError):
            recovery_tail([[10.0] * 35] * 3, float("inf"))

    def test_flat_and_warming_caches_pass(self):
        sample = rows()
        for index, row in enumerate(sample):
            row["rss_bytes"] += 64 * 1024**2 if index >= 5 else index * 1024**2
        self.assertEqual(assess(sample)["status"], "pass")

    def test_isolated_peak_and_cache_step_are_not_sustained_growth(self):
        sample = rows()
        for row in sample[18:]:
            row["rss_bytes"] *= 2
        sample[10]["rss_bytes"] *= 8
        self.assertEqual(assess(sample)["status"], "pass")

    def test_growth_and_slowdown_fail(self):
        for key, step in [
            ("rss_bytes", 4 * 1024**2),
            ("threads", 1),
            ("handles", 2),
            ("completion_ms", 2),
            ("definition_ms", 2),
        ]:
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
