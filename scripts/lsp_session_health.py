"""Compare equivalent settled checkpoints, allowing warmup and allocator plateaus."""

import math
import statistics


def resources(pid):
    import psutil
    process = psutil.Process(pid)
    with process.oneshot():
        return {"rss_bytes": process.memory_info().rss,
                "threads": process.num_threads(),
                "handles": process.num_handles() if hasattr(process, "num_handles") else process.num_fds()}


def assess(rows, warmup=5):
    """Require sustained growth across three windows, not a single cache step."""
    samples = rows[warmup:]
    if len(samples) < 15:
        raise ValueError("session needs at least 15 measured checkpoints after warmup")
    limits = {"rss_bytes": (32 * 1024 * 1024, 1.2), "threads": (4, 1.0),
              "handles": (8, 1.0), "completion_ms": (10, 2.0), "definition_ms": (10, 2.0)}
    width = len(samples) // 3
    windows = [samples[:width], samples[width:2 * width], samples[2 * width:]]
    measurements, failures = {}, []
    for key, (absolute, ratio) in limits.items():
        values = [row[key] for row in samples]
        if any(not math.isfinite(value) or value < 0 for value in values):
            raise ValueError(f"invalid checkpoint metric: {key}")
        medians = [statistics.median(row[key] for row in window) for window in windows]
        first, middle, last = medians
        slopes = [(values[j] - values[i]) / (j - i)
                  for i in range(len(values)) for j in range(i + 1, len(values))]
        slope = statistics.median(slopes)
        growing = (last - first > absolute and last > first * ratio
                   and middle - first > absolute / 4 and last - middle > absolute / 4
                   and slope > 0)
        measurements[key] = {"window_medians": medians, "slope_per_cycle": slope,
                             "absolute_tolerance": absolute, "ratio": ratio}
        if growing:
            failures.append(key)
    return {"status": "regression" if failures else "pass", "failures": failures,
            "warmup_cycles": warmup, "measured_cycles": len(samples), "metrics": measurements}
