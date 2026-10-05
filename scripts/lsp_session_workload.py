"""Repeated document lifecycle operations against one owned language server."""

import hashlib
import json
import statistics
import time

from lsp_measurement import change_event


class Session:
    def __init__(self, probe, binary, root, config, trace):
        self.client = probe.Client(binary, config)
        self.root, self.trace = root, trace
        self.sources = sorted((root / "src").glob("*.recite"))[:4]
        self.originals = {path: path.read_text() for path in self.sources}
        self.documents = {}
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
        result = self.client.send(method, params, request)
        self.trace.write(json.dumps({"pid": self.client.process.pid, "id": result[0],
                                     "method": method, "params": params, "request": request}) + "\n")
        self.trace.flush()
        return result

    def open(self, path):
        text = path.read_text()
        self.documents[path] = (1, text)
        self.send("textDocument/didOpen", {"textDocument": {
            "uri": path.as_uri(), "languageId": "recite", "version": 1, "text": text}})
        self.wait(path)

    def change(self, path, text, wait=True):
        version, previous = self.documents[path]
        self.documents[path] = (version + 1, text)
        self.send("textDocument/didChange", {"textDocument": {"uri": path.as_uri(), "version": version + 1},
                                            "contentChanges": [change_event(previous, text, True)]})
        if wait:
            return self.wait(path)

    def wait(self, path):
        version, _ = self.documents[path]
        self.client.diagnostics(path.as_uri(), version, timeout=10)
        diagnostic = next(item["params"]["diagnostics"] for item in reversed(self.client.notifications)
                          if item.get("method") == "textDocument/publishDiagnostics"
                          and item["params"].get("uri") == path.as_uri()
                          and item["params"].get("version") == version)
        self.client.notifications.clear()
        return diagnostic

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
        encoded = json.dumps(results, sort_keys=True).encode()
        values["result_sha256"] = hashlib.sha256(encoded).hexdigest()
        self.client.notifications.clear()
        return values

    def cycle(self, cycle, churn, edits, random):
        path = self.sources[1 + cycle % (len(self.sources) - 1)]
        self.open(path)
        original = self.originals[path]
        broken = original + f"\n:: session_broken\n-> missing_{cycle}\n"
        assert self.change(path, broken), "broken reference produced no diagnostic"
        assert not self.change(path, original), "repair failed to clear diagnostics"
        # Keep actual 5 ms edit bursts and recovery timing; skip human think time.
        pending = {}
        for index in range(edits):
            self.change(path, original + f"\n# cycle {cycle} edit {index} seed {random.randrange(1_000_000)}\n", wait=False)
            if index % 10 == 0:
                request_id, _ = self.send("textDocument/completion", {"textDocument": {"uri": self.main.as_uri()},
                    "position": self.completion}, request=True)
                pending[request_id] = False
                request_id, _ = self.send("textDocument/rename", {"textDocument": {"uri": self.main.as_uri()},
                    "position": self.declaration, "newName": "session_cancelled"}, request=True)
                pending[request_id] = True
                self.send("$/cancelRequest", {"id": request_id})
            time.sleep(0.005)
        while pending:
            item = self.client.messages.get(timeout=10)
            if isinstance(item, Exception):
                raise item
            _, message = item
            if "id" not in message:
                self.client.notifications.append(message)
                continue
            cancelled = pending.pop(message["id"])
            error = message.get("error")
            if error:
                assert (cancelled and error["code"] == -32800) or (
                    error["code"] == -32803 and error.get("data", {}).get("reason") == "stale_snapshot"), message
            else:
                assert message.get("result"), message
        started = time.perf_counter_ns()
        assert not self.change(path, original)
        request_id, _ = self.send("textDocument/rename", {"textDocument": {"uri": self.main.as_uri()},
            "position": self.declaration, "newName": "session_rename"}, request=True)
        self.send("$/cancelRequest", {"id": request_id})
        _, response = self.client.response(request_id, timeout=10)
        # Cancellation can lose the race to a correct response; never accept stale success.
        assert response.get("error", {}).get("code") == -32800 or response.get("result"), response
        self.query("textDocument/completion", self.reference)
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
        temporary = self.root / "src" / f"session-{cycle if churn else 0}.recite"
        temporary.write_text(f":: session_temporary_{cycle if churn else 0}\n-> END\n", newline="\n")
        self.watched(temporary, 1)
        self.open(temporary)
        self.close(temporary)
        temporary.unlink()
        self.watched(temporary, 3)
        manifest = self.root / "recite.project.toml"
        configuration = manifest.read_text()
        manifest.write_text(configuration.replace('content_set = "lsp-fanout"',
                                                  'content_set = "session-config"'), newline="\n")
        self.watched(manifest)
        manifest.write_text(configuration, newline="\n")
        self.watched(manifest)
        assert list(self.documents) == [self.main]
        assert all(path.read_text() == text for path, text in self.originals.items())
        return recovery
