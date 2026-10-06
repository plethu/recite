"""Repeated document lifecycle operations against one owned language server."""

import hashlib
import json
import statistics
import time

from lsp_measurement import change_event
from lsp_session_timing import Timing


class Session:
    def __init__(self, probe, binary, root, config, trace, *, server_env=None):
        self.client = probe.Client(binary, config, server_env=server_env)
        self.timing = Timing(self.client.process.pid)
        self.root, self.trace = root, trace
        self.sources = sorted((root / "src").glob("*.recite"))[:4]
        self.originals = {path: path.read_text() for path in self.sources}
        self.documents = {}
        self.errors = {}
        self.main = self.sources[0]
        self.reference = probe.position(self.originals[self.main], r"-> src/[^\n]+::(scene_\d+_\d+)")
        self.completion = probe.position(self.originals[self.main], r"-> (END)")
        self.declaration = probe.position(self.originals[self.main], r"^:: (scene_000_000)")

    def start(self):
        _, response = self.client.request("initialize", {"rootUri": self.root.as_uri(), "capabilities": {}})
        assert response["result"]["capabilities"]["textDocumentSync"]["change"] == 2
        self.client.send("initialized", {})
        self.open(self.main)

    def send(self, method, params, request=False):
        with self.timing.measure("driver_send_ms"):
            result = self.client.send(method, params, request)
        with self.timing.measure("driver_trace_ms"):
            self.trace.write(json.dumps({"pid": self.client.process.pid, "id": result[0],
                                         "event": "send", "started_ns": result[1],
                                         "method": method, "params": params, "request": request}) + "\n")
            self.trace.flush()
        return result

    def received(self, received, response):
        self.trace.write(json.dumps({"event": "response", "pid": self.client.process.pid,
                                     "id": response["id"], "received_ns": received}) + "\n")

    def open(self, path):
        text = path.read_text()
        self.documents[path] = (1, text)
        self.send("textDocument/didOpen", {"textDocument": {
            "uri": path.as_uri(), "languageId": "recite", "version": 1, "text": text}})
        self.wait(path)

    def change(self, path, text, wait=True):
        version, previous = self.documents[path]
        self.documents[path] = (version + 1, text)
        with self.timing.measure("driver_edit_build_ms"):
            edit = change_event(previous, text, True)
        _, self.last_edit_started = self.send("textDocument/didChange", {
            "textDocument": {"uri": path.as_uri(), "version": version + 1},
            "contentChanges": [edit]})
        if wait:
            return self.wait(path)

    def wait(self, path):
        version, _ = self.documents[path]
        self.last_diagnostic_received = self.client.diagnostics(path.as_uri(), version, timeout=10)
        diagnostic = next(item["params"]["diagnostics"] for item in reversed(self.client.notifications)
                          if item.get("method") == "textDocument/publishDiagnostics"
                          and item["params"].get("uri") == path.as_uri()
                          and item["params"].get("version") == version)
        self.collect_diagnostics()
        return diagnostic

    def collect_diagnostics(self):
        for message in self.client.notifications:
            if message.get("method") == "textDocument/publishDiagnostics":
                params = message["params"]
                if params["diagnostics"]:
                    self.errors[params["uri"]] = params["diagnostics"]
                else:
                    self.errors.pop(params["uri"], None)
        self.client.notifications.clear()

    def close(self, path):
        self.send("textDocument/didClose", {"textDocument": {"uri": path.as_uri()}})
        del self.documents[path]
        self.fence()

    def fence(self):
        # A versioned diagnostic is the barrier, not a guessed settling sleep.
        assert not self.change(self.main, self.originals[self.main])

    def watched(self, path, kind=2):
        self.send("workspace/didChangeWatchedFiles", {"changes": [{"uri": path.as_uri(), "type": kind}]})
        self.fence()

    def query(self, method, position):
        request_id, started = self.send(method, {"textDocument": {"uri": self.main.as_uri()},
                                                "position": position}, request=True)
        received, response = self.client.response(request_id, timeout=10)
        self.last_query_resume_ms = (time.perf_counter_ns() - received) / 1e6
        self.received(received, response)
        self.last_query_id = request_id
        assert "result" in response and "error" not in response, response
        assert response["result"], response
        return (received - started) / 1e6, response["result"]

    def checkpoint(self):
        values, results = {}, {}
        for label, method in [("completion", "textDocument/completion"), ("definition", "textDocument/definition")]:
            samples = []
            for _ in range(7):
                elapsed, result = self.query(method, self.completion if label == "completion" else self.reference)
                samples.append(elapsed)
            values[label + "_ms"] = statistics.median(samples)
            values[label + "_samples_ms"] = samples
            results[label] = result
        # Separate invocations use fresh temporary roots. Keep relative target
        # identity while making fingerprints comparable across server binaries.
        encoded = json.dumps(results, sort_keys=True).replace(self.root.as_uri(), "$PROJECT").encode()
        values["result_sha256"] = hashlib.sha256(encoded).hexdigest()
        self.collect_diagnostics()
        assert not self.errors, f"settled project retains diagnostics: {self.errors}"
        values["diagnostics"] = self.errors.copy()
        return values

    def cycle(self, cycle, churn, edits, random, *, edit_interval_ms=5, editing_cpu_accounting=False):
        self.timing.reset()
        path = self.sources[1 + cycle % (len(self.sources) - 1)]
        self.open(path)
        original = self.originals[path]
        broken = original + f"\n:: session_broken\n-> missing_{cycle}\n"
        assert self.change(path, broken), "broken reference produced no diagnostic"
        assert not self.change(path, original), "repair failed to clear diagnostics"
        self.timing.stage("open_break_repair_ms")
        # Use deadlines so an oversleep does not shift every subsequent edit.
        # Record actual send intervals: hosted timers cannot promise 5 ms wakeups.
        pending = {}
        burst_cpu = self.timing.cpu_seconds() if editing_cpu_accounting else None
        burst_started = time.perf_counter_ns()
        sent_at = []
        for index in range(edits):
            remaining = (burst_started + index * edit_interval_ms * 1_000_000 - time.perf_counter_ns()) / 1e9
            if remaining > 0:
                with self.timing.measure("driver_sleep_ms"):
                    time.sleep(remaining)
            self.change(path, original + f"\n# cycle {cycle} edit {index} seed {random.randrange(1_000_000)}\n", wait=False)
            sent_at.append(self.last_edit_started)
            if index % 10 == 0:
                request_id, _ = self.send("textDocument/completion", {"textDocument": {"uri": self.main.as_uri()},
                    "position": self.completion}, request=True)
                pending[request_id] = False
                request_id, _ = self.send("textDocument/rename", {"textDocument": {"uri": self.main.as_uri()},
                    "position": self.declaration, "newName": "session_cancelled"}, request=True)
                pending[request_id] = True
                self.send("$/cancelRequest", {"id": request_id})
        self.timing.stage("burst_ms")
        intervals = [(right - left) / 1e6 for left, right in zip(sent_at, sent_at[1:])]
        self.timing.parts["edit_intervals_ms"] = intervals
        self.timing.parts["burst_send_span_ms"] = (sent_at[-1] - sent_at[0]) / 1e6
        started = self.last_edit_started
        self.timing.parts["final_edit_to_drain_start_ms"] = (time.perf_counter_ns() - started) / 1e6
        while pending:
            remaining = 0.5 - (time.perf_counter_ns() - started) / 1e9
            assert remaining > 0, "burst requests failed to drain within 500 ms"
            item = self.client.messages.get(timeout=remaining)
            if isinstance(item, Exception):
                raise item
            received, message = item
            if "id" not in message:
                self.client.notifications.append(message)
                continue
            cancelled = pending.pop(message["id"])
            self.received(received, message)
            error = message.get("error")
            if error:
                assert "result" not in message, message
                assert (cancelled and error["code"] == -32800) or (
                    error["code"] == -32803 and error.get("data", {}).get("reason") == "stale_snapshot"), message
            else:
                assert message.get("result"), message
        self.timing.stage("drain_ms")
        assert not self.change(path, original)
        assert self.last_diagnostic_received is not None
        repair_received = self.last_diagnostic_received
        self.timing.parts["repair_main_resume_ms"] = (time.perf_counter_ns() - self.last_diagnostic_received) / 1e6
        self.timing.stage("repair_diagnostics_ms")
        if editing_cpu_accounting:
            self.timing.parts["editing_server_cpu_ms"] = (self.timing.cpu_seconds() - burst_cpu) * 1000
        request_id, rename_started = self.send("textDocument/rename", {"textDocument": {"uri": self.main.as_uri()},
            "position": self.declaration, "newName": "session_rename"}, request=True)
        self.send("$/cancelRequest", {"id": request_id})
        received, response = self.client.response(request_id, timeout=10)
        self.timing.parts["rename_main_resume_ms"] = (time.perf_counter_ns() - received) / 1e6
        self.received(received, response)
        # Cancellation can lose the race to a correct response; never accept stale success.
        assert response.get("error", {}).get("code") == -32800 or response.get("result"), response
        self.timing.stage("cancelled_rename_ms")
        self.query("textDocument/completion", self.reference)
        self.timing.parts["completion_main_resume_ms"] = self.last_query_resume_ms
        self.timing.stage("recovery_completion_ms")
        recovery = (time.perf_counter_ns() - started) / 1e6
        assert recovery < 500, f"recovery exceeded 500 ms: {recovery}"
        saved = original + "\n# saved session edit\n"
        assert not self.change(path, saved)
        path.write_text(saved, newline="\n")
        self.send("textDocument/didSave", {"textDocument": {"uri": path.as_uri()}})
        self.close(path)
        self.open(path)
        assert self.documents[path][1] == saved
        path.write_text(original, newline="\n")
        assert not self.change(path, original)
        self.send("textDocument/didSave", {"textDocument": {"uri": path.as_uri()}})
        self.close(path)
        self.timing.stage("save_close_reopen_ms")
        temporary = self.root / "src" / f"session-{cycle if churn else 0}.recite"
        temporary.write_text(f":: session_temporary_{cycle if churn else 0}\n-> END\n", newline="\n")
        self.watched(temporary, 1)
        self.open(temporary)
        self.close(temporary)
        temporary.unlink()
        self.watched(temporary, 3)
        self.timing.stage("temporary_document_ms")
        manifest = self.root / "recite.project.toml"
        configuration = manifest.read_text()
        manifest.write_text(configuration.replace('content_set = "lsp-fanout"',
                                                  'content_set = "session-config"'), newline="\n")
        self.watched(manifest)
        manifest.write_text(configuration, newline="\n")
        self.watched(manifest)
        self.timing.stage("configuration_ms")
        assert list(self.documents) == [self.main]
        assert all(path.read_text() == text for path, text in self.originals.items())
        self.timing.parts["repair_to_rename_send_ms"] = (rename_started - repair_received) / 1e6
        return {"recovery_ms": recovery, "timing": self.timing.finish(),
                "recovery_requests": {"rename": request_id, "completion": self.last_query_id}}
