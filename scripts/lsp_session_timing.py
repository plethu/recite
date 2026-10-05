"""Account for driver work separately from elapsed protocol stages."""

from collections import defaultdict
from contextlib import contextmanager
import time


class Timing:
    def __init__(self, pid):
        import psutil
        self.process = psutil.Process(pid)
        self.reset()

    def reset(self):
        self.parts = defaultdict(float)
        self.started = self.marked = time.perf_counter_ns()
        self.driver_cpu = time.process_time()
        self.server_cpu = self.cpu_seconds()

    def cpu_seconds(self):
        value = self.process.cpu_times()
        return value.user + value.system

    @contextmanager
    def measure(self, name):
        started = time.perf_counter_ns()
        try:
            yield
        finally:
            self.parts[name] += (time.perf_counter_ns() - started) / 1e6

    def stage(self, name):
        now = time.perf_counter_ns()
        self.parts[name] = (now - self.marked) / 1e6
        self.marked = now

    def finish(self):
        return {**self.parts, "cycle_ms": (time.perf_counter_ns() - self.started) / 1e6,
                "driver_cpu_ms": (time.process_time() - self.driver_cpu) * 1000,
                "server_cpu_ms": (self.cpu_seconds() - self.server_cpu) * 1000}
