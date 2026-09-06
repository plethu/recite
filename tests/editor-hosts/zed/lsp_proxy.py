#!/usr/bin/env python3
"""Log and forward one LSP stdio session without changing its wire bytes.

The proxy is intentionally transport-only.  It parses Content-Length framing
to record the JSON messages, then forwards each original frame byte-for-byte
between Zed and the real Recite server.  It does not synthesize requests,
responses, diagnostics, or edits.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
from typing import BinaryIO


LOG_PATH = os.environ["RECITE_PROBE_LSP_LOG"]
REAL_SERVER = os.environ["RECITE_PROBE_LSP_REAL"]
log_lock = threading.Lock()


def read_frame(stream: BinaryIO) -> tuple[bytes, bytes] | None:
    headers = bytearray()
    while True:
        line = stream.readline()
        if not line:
            return None
        headers.extend(line)
        if line in (b"\r\n", b"\n"):
            break

    content_length: int | None = None
    for line in bytes(headers).splitlines():
        name, separator, value = line.partition(b":")
        if separator and name.lower() == b"content-length":
            content_length = int(value.strip())
    if content_length is None:
        raise RuntimeError("LSP frame omitted Content-Length")
    body = stream.read(content_length)
    if len(body) != content_length:
        raise RuntimeError("LSP frame ended before Content-Length bytes")
    return bytes(headers) + body, body


def record(direction: str, body: bytes) -> None:
    message = json.loads(body.decode("utf-8"))
    entry = {"direction": direction, "message": message}
    with log_lock:
        with open(LOG_PATH, "a", encoding="utf-8") as log:
            json.dump(entry, log, sort_keys=True, separators=(",", ":"))
            log.write("\n")


def client_to_server(server: subprocess.Popen[bytes]) -> None:
    try:
        while True:
            frame = read_frame(sys.stdin.buffer)
            if frame is None:
                break
            raw, body = frame
            record("client->server", body)
            if server.stdin is None:
                break
            server.stdin.write(raw)
            server.stdin.flush()
    finally:
        if server.stdin is not None:
            server.stdin.close()


def server_to_client(server: subprocess.Popen[bytes]) -> None:
    try:
        if server.stdout is None:
            return
        while True:
            frame = read_frame(server.stdout)
            if frame is None:
                break
            raw, body = frame
            record("server->client", body)
            sys.stdout.buffer.write(raw)
            sys.stdout.buffer.flush()
    finally:
        try:
            sys.stdout.buffer.flush()
        except BrokenPipeError:
            pass


def main() -> int:
    server = subprocess.Popen(
        [REAL_SERVER, *sys.argv[1:]],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    client_thread = threading.Thread(target=client_to_server, args=(server,))
    server_thread = threading.Thread(target=server_to_client, args=(server,))
    client_thread.start()
    server_thread.start()
    status = server.wait()
    client_thread.join(timeout=2)
    server_thread.join(timeout=2)
    return status


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (BrokenPipeError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"recite-lsp probe transport failure: {error}", file=sys.stderr)
        raise SystemExit(1)
