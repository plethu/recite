#!/usr/bin/env python3
"""Exercise an extracted Linux writer package through a private desktop session.

Evidence covers desktop association/argv delivery and the real app's IPC ack;
it does not cover human usability or package-manager upgrade/uninstall.
"""
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from urllib.parse import urlencode
LIMIT = 4096

def require(ok, message):
    if not ok:
        raise RuntimeError(message)

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
        time.sleep(.05)
    raise RuntimeError(f"timed out waiting for {what}; launcher evidence: {events(log)}")


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
            [os.environ["RECITE_SMOKE_BINARY"], *argv], stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=stderr_file, start_new_session=True,
        )
    except OSError as error:
        stderr_file.close()
        record(log, {"event": "spawn-error", "pid": pid, "error": str(error)})
        return 127
    record(log, {"event": "started", "pid": pid, "child": child.pid, "argv": argv, "stderr": str(stderr_path)})
    while True:
        try:
            status = child.wait(timeout=.1)
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

def fixture(base):
    project = base / "fixture project — Café"
    project.mkdir()
    files = {
        "recite.project.toml": 'format_version = 1\n[project]\ncontent_set = "desktop-smoke"\nversion = "1"\n',
        "first scene.recite": ":: start default\n> cold_open@0123456789abcdef0123\n  Cold launch fixture.\n-> END\n",
        "second scene λ.recite": ":: start default\n> second_scene@abcdef0123456789abcd\n  Running instance fixture.\n-> END\n",
    }
    snapshot = {}
    for name, text in files.items():
        path = project / name
        snapshot[path] = text.encode()
        path.write_bytes(snapshot[path])
    return project, snapshot

def route(project, scene):
    return "recite://writer/write?" + urlencode({"project": str(project), "scene": scene})

def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def invoke(uri, env, timeout=15):
    require(len(uri.encode()) <= LIMIT, "test URI exceeds the ingress bound")
    with tempfile.TemporaryFile() as error_output:
        result = subprocess.run(["gio", "open", uri], env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.DEVNULL, stderr=error_output, timeout=timeout)
        error_output.seek(0)
        error = error_output.read().decode("utf-8", errors="replace").strip()
    require(result.returncode == 0, f"gio open failed: {error}")


def forwarded_scene(uri, env, log):
    busy = ("The writer is still opening a project.",
            "A project is still opening. Cancel it or wait for it to finish.")
    deadline = time.monotonic() + 20
    attempts = 0
    while time.monotonic() < deadline:
        before = len([e for e in events(log) if e.get("event") == "started" and e.get("argv") == [uri]])
        invoke(uri, env, max(.1, min(5, deadline - time.monotonic())))
        new_starts = wait_for(
            log,
            lambda es: [e for e in es if e.get("event") == "started" and e.get("argv") == [uri]][before:],
            min(15, max(.1, deadline - time.monotonic())),
            "second-route process",
        )
        start = new_starts[0]
        exits = wait_for(
            log,
            lambda es: [e for e in es if e.get("event") == "exited" and e.get("child") == start["child"]],
            min(15, max(.1, deadline - time.monotonic())),
            "second-route acknowledgement",
        )
        exited = exits[0]
        if exited["status"] == 0:
            return start, attempts
        stderr = Path(start["stderr"]).read_text(encoding="utf-8", errors="replace")
        if exited["status"] != 2 or not any(message in stderr for message in busy):
            raise RuntimeError(f"second-route process failed ({exited['status']}): {stderr.strip()}")
        attempts += 1
        if time.monotonic() + .1 >= deadline:
            break
        time.sleep(.1)
    raise RuntimeError(f"writer did not become UI-ready within 20 seconds after {attempts} bounded busy retries")

def smoke(root):
    root = root.resolve()
    binary, app_dir = root / "usr/bin/recite-writer", root / "usr/share/applications"
    require(binary.is_file() and os.access(binary, os.X_OK), f"missing executable: {binary}")
    matches = []
    for path in app_dir.glob("*.desktop"):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeError):
            continue
        mime = {x for line in lines if line.startswith("MimeType=") for x in line[9:].split(";")}
        if "Exec=recite-writer %u" in lines and "x-scheme-handler/recite" in mime:
            matches.append(path)
    require(len(matches) == 1, f"expected one Exec=recite-writer %u desktop entry with recite scheme; found {matches}")
    require(shutil.which("gio") and shutil.which("dbus-run-session"), "gio and dbus-run-session are required")
    require(os.environ.get("DISPLAY") or os.environ.get("WAYLAND_DISPLAY"), "a DISPLAY or WAYLAND_DISPLAY is required")

    with tempfile.TemporaryDirectory(prefix="recite-desktop-links-") as temp:
        base = Path(temp)
        dirs = {name: base / name for name in ("config", "data", "state", "runtime", "bin")}
        for path in dirs.values():
            path.mkdir(mode=0o700)
            path.chmod(0o700)
        apps = dirs["data"] / "applications"
        apps.mkdir(mode=0o700)
        shutil.copyfile(matches[0], apps / matches[0].name)
        (dirs["config"] / "mimeapps.list").write_text(
            f"[Default Applications]\nx-scheme-handler/recite={matches[0].name};\n", encoding="utf-8"
        )
        project, snapshot = fixture(base)
        conf = base / "session.conf"
        conf.write_text(
            '<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen>'
            '<policy context="default"><allow send_destination="*"/>'
            '<allow receive_sender="*"/><allow own="*"/></policy></busconfig>', encoding="utf-8"
        )
        log, stop = base / "launcher.jsonl", base / "stop"
        script = Path(__file__).resolve()
        wrapper = dirs["bin"] / "recite-writer"
        wrapper.write_text(
            f"#!{sys.executable}\nimport os,sys\n"
            f"os.execv({sys.executable!r},[{sys.executable!r},{str(script)!r},'--launcher',*sys.argv[1:]])\n",
            encoding="utf-8",
        )
        wrapper.chmod(0o700)
        env = {
            **os.environ, "XDG_CONFIG_HOME": str(dirs["config"]),
            "RECITE_CONFIG": str(dirs["config"] / "recite.toml"),
            "XDG_DATA_HOME": str(dirs["data"]), "XDG_STATE_HOME": str(dirs["state"]),
            "XDG_RUNTIME_DIR": str(dirs["runtime"]), "XDG_DATA_DIRS": "/usr/local/share:/usr/share",
            "GSETTINGS_BACKEND": "memory", "GTK_USE_PORTAL": "0", "GIO_USE_PORTALS": "0",
            "RECITE_SMOKE_LOG": str(log), "RECITE_SMOKE_STOP": str(stop),
            "RECITE_SMOKE_BINARY": str(binary), "PATH": str(dirs["bin"]) + os.pathsep + os.environ.get("PATH", "/usr/bin:/bin"),
        }
        wayland, old_runtime = os.environ.get("WAYLAND_DISPLAY"), os.environ.get("XDG_RUNTIME_DIR")
        if wayland and not os.path.isabs(wayland):
            require(old_runtime, "relative WAYLAND_DISPLAY requires the original XDG_RUNTIME_DIR")
            env["WAYLAND_DISPLAY"] = str((Path(old_runtime) / wayland).resolve())
        env.pop("DBUS_SESSION_BUS_ADDRESS", None)
        env.pop("AT_SPI_BUS_ADDRESS", None)
        bus = None
        try:
            bus = subprocess.Popen(
                ["dbus-run-session", "--config-file", str(conf), "--", sys.executable,
                 str(script), "--inside", str(root)], env=env, stdin=subprocess.DEVNULL,
                start_new_session=True,
            )
            deadline = time.monotonic() + 90
            while bus.poll() is None and time.monotonic() < deadline:
                time.sleep(.05)
            require(bus.poll() is not None, "smoke exceeded 90-second overall bound")
            require(bus.returncode == 0, f"private desktop smoke exited {bus.returncode}")
        finally:
            stop.touch(mode=0o600, exist_ok=True)
            if bus and bus.poll() is None:
                bus.terminate()
                try:
                    bus.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    bus.kill()
                    bus.wait()
            started = [e for e in events(log) if e.get("event") == "started"]
            if started:
                wait_for(log, lambda es: len([e for e in es if e.get("event") == "exited"]) >= len(started), 5,
                         "owned launched writers to exit during cleanup")
        changed = [str(path) for path, content in snapshot.items() if path.read_bytes() != content]
        require(not changed, f"fixture bytes changed: {changed}")
        print("PASS: actual package desktop entry and binary dispatched through gio.")
        print("PASS: cold route launched; running-window route returned 0; malformed route returned 2.")
        print("PASS: fixture bytes unchanged. Human usability and install/upgrade/uninstall are separate gates.")


def inside(root):
    base = Path(os.environ["XDG_STATE_HOME"]).parent
    log = Path(os.environ["RECITE_SMOKE_LOG"])
    mime = subprocess.run(["gio", "mime", "x-scheme-handler/recite"], stdin=subprocess.DEVNULL,
                          capture_output=True, text=True, timeout=15)
    require(mime.returncode == 0 and next((Path(os.environ["XDG_DATA_HOME"]) / "applications").glob("*.desktop")).name in mime.stdout,
            f"temporary scheme handler was not registered with gio: {mime.stderr.strip()} {mime.stdout.strip()}")
    project = base / "fixture project — Café"
    first, second = route(project, "first scene.recite"), route(project, "second scene λ.recite")
    invoke(first, os.environ.copy())
    start1 = wait_for(log, lambda es: [e for e in es if e.get("event") == "started"], 30, "cold launch")[0]
    require(start1["argv"] == [first], f"desktop argv changed: {start1}")
    first_pid = int(start1["child"])
    wait_for(log, lambda _: alive(first_pid), 30, "cold app to remain running")
    require(alive(first_pid), "cold app exited before activation")
    start2, retries = forwarded_scene(second, os.environ.copy(), log)
    require(alive(first_pid), "the primary writer exited during running-window delivery")
    bad = "recite://writer/not-a-supported-screen"
    invoke(bad, os.environ.copy())
    start3 = wait_for(log, lambda es: [e for e in es if e.get("event") == "started" and e.get("argv") == [bad]], 30, "malformed launch")[0]
    exit3 = wait_for(log, lambda es: [e for e in es if e.get("event") == "exited" and e.get("child") == start3["child"]], 15, "malformed route rejection")[0]
    require(exit3["status"] == 2 and alive(first_pid), f"malformed route was not rejected: {exit3}")
    require(
        len([e for e in events(log) if e.get("event") == "started"]) == 3 + retries,
        "unexpected app launch outside the bounded UI-readiness retries",
    )
    print(f"Desktop argv observed; packaged writer PIDs: {[start1['child'], start2['child'], start3['child']]}; UI-readiness retries: {retries}")

def main():
    if len(sys.argv) >= 2 and sys.argv[1] == "--launcher":
        return launch_wrapper()
    if len(sys.argv) == 3 and sys.argv[1] == "--inside":
        inside(Path(sys.argv[2]))
        return 0
    if len(sys.argv) != 2 or sys.platform != "linux":
        print(f"usage: {Path(sys.argv[0]).name} EXTRACTED_DEB_ROOT (Linux only)", file=sys.stderr)
        return 2
    try:
        smoke(Path(sys.argv[1]))
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
