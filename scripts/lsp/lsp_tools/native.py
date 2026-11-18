"""Join native monotonic intervals with driver durations by process and request ID."""

import json
import math
import statistics
from collections import defaultdict


def distribution(values):
    if not values or any(not math.isfinite(value) for value in values):
        raise ValueError("require finite timing samples")
    return {
        "count": len(values),
        "median": statistics.median(values),
        "p95": sorted(values)[math.ceil(len(values) * 0.95) - 1],
        "min": min(values),
        "max": max(values),
    }


def native_requests(path):
    requests, jobs = defaultdict(dict), defaultdict(dict)
    events = [json.loads(line) for line in path.read_text().splitlines()]
    for event in sorted(
        events, key=lambda event: float(event["timestamp"].strip().removesuffix("s"))
    ):
        fields = event["fields"]
        phase = fields["phase"]
        milliseconds = float(event["timestamp"].strip().removesuffix("s")) * 1000
        if "id" in fields:
            request = requests[str(fields["id"])]
            request[phase] = milliseconds
            if "serial" in fields:
                request["serial"] = fields["serial"]
        elif "serial" in fields:
            jobs[fields["serial"]][phase] = milliseconds
    for request in requests.values():
        if "serial" in request:
            request.update(jobs[request["serial"]])
    return requests


def intervals(native, wire_ms):
    result = {
        "wire_ms": wire_ms,
        "server_ms": native["handoff"] - native["ingress"],
        "ready_handoff_ms": native["handoff"] - native["output_ready"],
    }
    # Signed: the sending thread can resume after the driver has received the reply.
    result["outside_server_ms"] = wire_ms - result["server_ms"]
    for label, start, end in (
        ("queue_ms", "ingress", "query_dispatch"),
        ("worker_wake_ms", "query_dispatch", "query_start"),
        ("execution_ms", "query_start", "query_end"),
        ("result_wake_ms", "query_end", "query_observed"),
    ):
        if start in native and end in native:
            result[label] = native[end] - native[start]
    return result


def recovery_wire_samples(report, trace_path):
    sends, received = {}, {}
    for line in trace_path.read_text().splitlines():
        event = json.loads(line)
        if event["pid"] != report["server_pid"] or event.get("id") is None:
            continue
        if event["event"] == "send":
            sends[event["id"]] = event["started_ns"]
        elif event["event"] == "response":
            received[event["id"]] = event["received_ns"]
    samples = defaultdict(list)
    for checkpoint in report["checkpoints"][5:]:
        for operation, request_id in checkpoint["recovery_requests"].items():
            elapsed = (received[request_id] - sends[request_id]) / 1e6
            if not math.isfinite(elapsed) or elapsed < 0:
                raise ValueError("invalid client wire interval")
            samples[operation].append(elapsed)
    return samples


def recovery_samples(report_path):
    report = json.loads(report_path.read_text())
    if report["status"] != "pass" or len(report["checkpoints"]) != report["cycles"]:
        raise ValueError("native attribution requires a complete passing session")
    pid = report["server_pid"]
    native = native_requests(report_path.with_suffix("") / f"{pid}.jsonl")
    sends, received = {}, {}
    for line in report_path.with_suffix(".jsonl").read_text().splitlines():
        event = json.loads(line)
        if event["pid"] != pid or event["id"] is None:
            continue
        if event["event"] == "send":
            sends[event["id"]] = event["started_ns"]
        else:
            received[event["id"]] = event["received_ns"]
    samples = defaultdict(list)
    for checkpoint in report["checkpoints"][5:]:
        for operation, request_id in checkpoint["recovery_requests"].items():
            wire = (received[request_id] - sends[request_id]) / 1e6
            samples[operation].append(intervals(native[str(request_id)], wire))
    return samples


def summarize_samples(samples):
    keys = sorted({key for sample in samples for key in sample})
    return {key: distribution([sample[key] for sample in samples if key in sample]) for key in keys}
