"""Owned stdio client and source positions for external LSP measurements."""

import json
import os
import queue
import re
import subprocess
import tempfile
import threading
import time


class Client:
    def __init__(self, binary, config_root, *, server_env=None):
        self.stderr = tempfile.TemporaryFile()
        self.process = subprocess.Popen(
            [str(binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.stderr,
            env={
                **os.environ,
                "XDG_CONFIG_HOME": str(config_root),
                "APPDATA": str(config_root),
                "LOCALAPPDATA": str(config_root),
                **(server_env or {}),
            },
        )
        self.messages = queue.Queue()
        self.next_id = 0
        self.notifications = []
        self.reader = threading.Thread(target=self.read, daemon=True)
        self.reader.start()

    def read(self):
        try:
            while True:
                headers = {}
                while True:
                    line = self.process.stdout.readline()
                    if not line:
                        raise EOFError("server stdout closed")
                    if line == b"\r\n":
                        break
                    key, value = line.decode("ascii").split(":", 1)
                    headers[key.lower()] = value.strip()
                body = self.process.stdout.read(int(headers["content-length"]))
                self.messages.put((time.perf_counter_ns(), json.loads(body)))
        except Exception as error:
            self.messages.put(error)

    def send(self, method, params, request=False):
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        if request:
            self.next_id += 1
            message["id"] = self.next_id
        body = json.dumps(message, separators=(",", ":")).encode()
        started = time.perf_counter_ns()
        self.process.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
        self.process.stdin.flush()
        return message.get("id"), started

    def response(self, request_id, timeout=120):
        deadline = time.monotonic() + timeout
        while True:
            item = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            if isinstance(item, Exception):
                raise item
            received, message = item
            if message.get("id") == request_id:
                return received, message
            if "id" in message:
                raise RuntimeError(f"unexpected server request or response: {message}")
            self.notifications.append(message)

    def request(self, method, params, timeout=120):
        request_id, started = self.send(method, params, request=True)
        received, message = self.response(request_id, timeout)
        return (received - started) / 1e6, message

    def diagnostics(self, uri, version, timeout=120):
        """Return the reader timestamp for newly received diagnostics, else None."""

        def matches(message):
            return (
                message.get("method") == "textDocument/publishDiagnostics"
                and message["params"]["uri"] == uri
                and message["params"].get("version") == version
            )

        if any(matches(message) for message in self.notifications):
            return
        deadline = time.monotonic() + timeout
        while True:
            item = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            if isinstance(item, Exception):
                raise item
            received, message = item
            if "id" in message:
                raise RuntimeError(f"unexpected response while waiting for diagnostics: {message}")
            self.notifications.append(message)
            if matches(message):
                return received

    def close(self):
        try:
            if self.process.poll() is None:
                self.request("shutdown", None)
                self.send("exit", None)
                self.process.stdin.close()
                self.process.wait(timeout=10)
            if self.process.returncode != 0:
                self.stderr.seek(0)
                raise RuntimeError(self.stderr.read().decode())
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait()
            self.process.stdout.close()
            self.reader.join(timeout=1)
            self.stderr.close()

    def abort(self):
        """Clean up an owned server without masking a failed probe's exception."""
        if self.process.poll() is None:
            self.process.kill()
        self.process.wait()
        self.process.stdin.close()
        self.process.stdout.close()
        self.reader.join(timeout=1)
        self.stderr.close()


def position(text, pattern, end=False):
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        raise ValueError(f"fixture lacks probe {pattern}")
    offset = match.end(1) if end else match.start(1)
    prefix = text[:offset]
    return {
        "line": prefix.count("\n"),
        "character": len(prefix.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2,
    }
