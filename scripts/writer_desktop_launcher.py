"""Owned package-launch processes, bounded argument ingress and diagnostic receipts."""

import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

LIMIT = 4096


def events(log):
    try:
        content = log.read_bytes()
    except FileNotFoundError:
        return []
    complete = content.rsplit(b"\n", 1)[0] if not content.endswith(b"\n") else content[:-1]
    if not complete:
        return []
    return [json.loads(line.decode("utf-8")) for line in complete.split(b"\n")]


def wait_for(log, select, seconds, what):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        result = select(events(log))
        if result:
            return result
        time.sleep(0.05)
    raise RuntimeError(f"timed out waiting for {what}; launcher evidence: {diagnostics(log)}")


def diagnostics(log):
    evidence = events(log)
    stderr = {}
    for event in evidence[-4:]:
        if "stderr" in event:
            try:
                with Path(event["stderr"]).open("rb") as source:
                    stderr[event["child"]] = source.read(LIMIT).decode("utf-8", errors="replace")
            except OSError as error:
                stderr[event["child"]] = str(error)
    return {"events": evidence, "stderr": stderr}


def record(log, value):
    fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, (json.dumps(value, ensure_ascii=False) + "\n").encode())
    finally:
        os.close(fd)


def launch_wrapper():
    argv = sys.argv[2:]
    log, stop = Path(os.environ["RECITE_SMOKE_LOG"]), Path(os.environ["RECITE_SMOKE_STOP"])
    if not argv or any(len(arg.encode()) > LIMIT for arg in argv):
        record(log, {"event": "rejected", "argv": argv})
        return 2
    pid = os.getpid()
    stderr_path = log.parent / f"writer-{pid}.stderr"
    stderr_file = stderr_path.open("wb")
    try:
        child = subprocess.Popen(
            [os.environ["RECITE_SMOKE_BINARY"], *argv],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=stderr_file,
            start_new_session=True,
        )
    except OSError as error:
        stderr_file.close()
        record(log, {"event": "spawn-error", "pid": pid, "error": str(error)})
        return 127
    record(
        log,
        {
            "event": "started",
            "pid": pid,
            "child": child.pid,
            "argv": argv,
            "stderr": str(stderr_path),
        },
    )
    while True:
        try:
            status = child.wait(timeout=0.1)
            break
        except subprocess.TimeoutExpired:
            if stop.exists():
                child.send_signal(signal.SIGTERM)
                try:
                    status = child.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    child.kill()
                    status = child.wait()
                break
    stderr_file.close()
    record(log, {"event": "exited", "pid": pid, "child": child.pid, "status": status})
    return status if status >= 0 else min(255, 128 - status)


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
