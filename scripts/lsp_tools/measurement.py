"""Shared measurement metadata and exact UTF-16 edit construction."""

import hashlib
import platform
import re
import subprocess
import time
from pathlib import Path


def change_event(before, after, ranged):
    if not ranged:
        return {"text": after}
    start = 0
    limit = min(len(before), len(after))
    while start < limit and before[start] == after[start]:
        start += 1
    end_before, end_after = len(before), len(after)
    while (
        end_before > start and end_after > start and before[end_before - 1] == after[end_after - 1]
    ):
        end_before -= 1
        end_after -= 1

    def splits_crlf(text, offset):
        return 0 < offset < len(text) and text[offset - 1 : offset + 1] == "\r\n"

    if splits_crlf(before, start) or splits_crlf(after, start):
        start -= 1
    if splits_crlf(before, end_before) or splits_crlf(after, end_after):
        end_before += 1
        end_after += 1

    def position(offset):
        prefix = before[:offset]
        breaks = list(re.finditer(r"\r\n|\r|\n", prefix))
        tail = prefix[breaks[-1].end() :] if breaks else prefix
        return {"line": len(breaks), "character": len(tail.encode("utf-16-le")) // 2}

    return {
        "range": {"start": position(start), "end": position(end_before)},
        "text": after[start:end_after],
    }


def environment():
    paths = [Path("/sys/firmware/acpi/platform_profile")]
    paths.extend(sorted(Path("/sys/class/power_supply").glob("*/online")))
    paths.extend(sorted(Path("/sys/devices/system/cpu").glob("cpu0/cpufreq/scaling_governor")))
    cpuinfo = Path("/proc/cpuinfo")
    cpu = (
        next(
            (
                line.split(":", 1)[1].strip()
                for line in cpuinfo.read_text().splitlines()
                if line.startswith("model name")
            ),
            "unknown",
        )
        if cpuinfo.exists()
        else platform.processor()
    )
    return {
        "platform": platform.platform(),
        "cpu": cpu,
        "power": {str(path): path.read_text().strip() for path in paths if path.exists()},
    }


def provenance(binary, root):
    return {
        "recorded_at_unix": time.time(),
        "environment": environment(),
        "harness_revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip(),
        "harness_dirty": bool(
            subprocess.check_output(["git", "status", "--porcelain"], text=True).strip()
        ),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "files": {
            str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(root.rglob("*"))
            if p.is_file()
        },
    }
