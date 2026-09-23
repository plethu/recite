#!/usr/bin/python3
"""Probe the Linux AT-SPI bridge in a private D-Bus session.

Creates its own dbus-run-session. Requires python3-gi and a graphical session (or
Xvfb). Only the child component specimen is inspected or controlled.
"""

import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import sys
import shutil
import tempfile
import time

import gi

gi.require_version("Gio", "2.0")
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, Gio, GLib


ACCESSIBLE = "org.a11y.atspi.Accessible"
ROOT = "/org/a11y/atspi/accessible/root"


def call(bus, address, interface, method, arguments=None):
    return bus.call_sync(
        address[0], address[1], interface, method, arguments, None,
        Gio.DBusCallFlags.NONE, 2000, None,
    ).unpack()


def property_value(bus, address, interface, name):
    return call(bus, address, "org.freedesktop.DBus.Properties", "Get",
                GLib.Variant("(ss)", (interface, name)))[0]


def wait_for(operation, description, process, timeout=20):
    deadline = time.monotonic() + timeout
    last_error = None
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Writer exited with {process.returncode}: {description}")
        try:
            value = operation()
            if value:
                return value
        except GLib.Error as error:
            last_error = error
        time.sleep(0.1)
    raise RuntimeError(f"Timed out: {description}; last bus error: {last_error}")


def descendants(bus, application):
    pending = [application]
    seen = set()
    deadline = time.monotonic() + 10
    while pending:
        if time.monotonic() > deadline:
            raise RuntimeError("Specimen accessibility tree exceeded the traversal deadline")
        address = tuple(pending.pop())
        if address in seen:
            continue
        seen.add(address)
        if len(seen) > 5000:
            raise RuntimeError("Specimen accessibility tree exceeds the probe bound")
        name = property_value(bus, address, ACCESSIBLE, "Name")
        yield address, name
        pending.extend(call(bus, address, ACCESSIBLE, "GetChildren")[0])


@contextmanager
def child_process(arguments, **kwargs):
    process = subprocess.Popen(arguments, **kwargs)
    try:
        yield process
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def probe(binary, require_actions):
    # The enclosing command creates this bus; do not toggle the desktop user's
    # accessibility settings or connect to their registry.
    if os.environ.get("RECITE_PRIVATE_A11Y_BUS") != "1":
        raise RuntimeError("Use the check-writer-native-accessibility just recipe")
    session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    status = ("org.a11y.Bus", "/org/a11y/bus")
    call(session, status, "org.freedesktop.DBus.Properties", "Set",
         GLib.Variant("(ssv)", ("org.a11y.Status", "IsEnabled", GLib.Variant("b", True))))
    address = call(session, status, "org.a11y.Bus", "GetAddress")[0]
    bus = Gio.DBusConnection.new_for_address_sync(
        address, Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT
        | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION, None, None,
    )
    registry = ("org.a11y.atspi.Registry", ROOT)
    registry_binary = next((path for path in [shutil.which("at-spi2-registryd"),
                           "/usr/lib/at-spi2-registryd", "/usr/libexec/at-spi2-registryd"]
                           if path and Path(path).is_file()), None)
    if registry_binary is None:
        raise RuntimeError("Install the AT-SPI registry daemon")
    # Start explicitly: some distributions wire D-Bus activation to systemd,
    # which does not run inside this private session.
    with child_process([registry_binary], env=dict(os.environ, AT_SPI_BUS_ADDRESS=address),
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) as registry_process:
        wait_for(lambda: call(bus, registry, ACCESSIBLE, "GetChildren") or True,
                 "private AT-SPI registry", registry_process)
        run_specimen(binary, bus, registry, require_actions)


def run_specimen(binary, bus, registry, require_actions):
    before = {tuple(child) for child in call(bus, registry, ACCESSIBLE, "GetChildren")[0]}
    with tempfile.TemporaryDirectory(prefix="recite-native-a11y-") as temporary:
        env = dict(os.environ, XDG_CONFIG_HOME=temporary, XDG_STATE_HOME=temporary)
        env["RECITE_CONFIG"] = str(Path(temporary) / "recite.toml")
        with open(Path(temporary) / "writer.log", "w+") as log:
            process = subprocess.Popen([str(binary), "--design-system"], env=env,
                                       stdout=log, stderr=subprocess.STDOUT)
            try:
                def find_application():
                    children = call(bus, registry, ACCESSIBLE, "GetChildren")[0]
                    for child in children:
                        if tuple(child) not in before:
                            # Only inspect the process launched by this probe.
                            pid = call(bus, ("org.freedesktop.DBus", "/org/freedesktop/DBus"),
                                       "org.freedesktop.DBus", "GetConnectionUnixProcessID",
                                       GLib.Variant("(s)", (child[0],)))[0]
                            if pid == process.pid:
                                return child
                    return None

                application = wait_for(find_application, "writer accessibility registration", process)

                def find_named(name, interface=None):
                    return next((node for node, label in descendants(bus, application)
                                 if label == name and (interface is None or interface in
                                     call(bus, node, ACCESSIBLE, "GetInterfaces")[0])), None)

                button = wait_for(lambda: next((node for node, label in descendants(bus, application)
                    if label == "Try scene" and call(bus, node, ACCESSIBLE, "GetRole")[0]
                    == int(Atspi.Role.PUSH_BUTTON)), None), "named primary button", process)
                interfaces = call(bus, button, ACCESSIBLE, "GetInterfaces")[0]
                action_available = "org.a11y.atspi.Action" in interfaces
                focus_requested = call(bus, button, "org.a11y.atspi.Component", "GrabFocus")[0]
                focused = False
                if focus_requested:
                    try:
                        focused = wait_for(lambda: bool(call(bus, button, ACCESSIBLE, "GetState")[0][0]
                                                       & (1 << int(Atspi.StateType.FOCUSED))),
                                           "native focus state", process, timeout=2)
                    except RuntimeError:
                        if process.poll() is not None:
                            raise
                activated = False
                if action_available:
                    accepted = call(bus, button, "org.a11y.atspi.Action", "DoAction",
                                    GLib.Variant("(i)", (0,)))[0]
                    if accepted:
                        try:
                            activated = bool(wait_for(lambda: find_named("Primary action activated."),
                                "native action reflected in the tree", process, timeout=2))
                        except RuntimeError:
                            if process.poll() is not None:
                                raise
                complete = focused and activated
                print(json.dumps({"platform": "linux", "bridge": "AT-SPI",
                                  "status": "passed" if complete else "limited",
                                  "named_button": True, "focus_observed": focused,
                                  "action_available": action_available, "action_observed": activated,
                                  "human_screen_reader_acceptance": False}), flush=True)
                if require_actions and not complete:
                    raise RuntimeError("Native accessibility action gate failed; see capability report")
            except Exception:
                log.flush()
                log.seek(0)
                print(log.read())
                raise
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--require-actions", action="store_true",
                        help="Fail unless native focus and activation both change the application")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    if os.environ.get("RECITE_PRIVATE_A11Y_BUS") == "1":
        launcher = next((path for path in [shutil.which("at-spi-bus-launcher"),
                         "/usr/lib/at-spi-bus-launcher", "/usr/libexec/at-spi-bus-launcher"]
                         if path and Path(path).is_file()), None)
        if launcher is None:
            raise RuntimeError("Install the AT-SPI bus launcher")
        with child_process([launcher, "--launch-immediately"]) as bus_process:
            session = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            wait_for(lambda: call(session, ("org.freedesktop.DBus", "/org/freedesktop/DBus"),
                                  "org.freedesktop.DBus", "NameHasOwner",
                                  GLib.Variant("(s)", ("org.a11y.Bus",)))[0],
                     "private accessibility bus", bus_process)
            probe(binary, args.require_actions)
    else:
        with tempfile.TemporaryDirectory(prefix="recite-a11y-session-") as temporary:
            env = dict(os.environ, RECITE_PRIVATE_A11Y_BUS="1",
                       XDG_CONFIG_HOME=temporary, XDG_STATE_HOME=temporary,
                       XDG_RUNTIME_DIR=temporary, GSETTINGS_BACKEND="memory")
            env["RECITE_CONFIG"] = str(Path(temporary) / "recite.toml")
            wayland = os.environ.get("WAYLAND_DISPLAY", "")
            if wayland and not Path(wayland).is_absolute():
                env["WAYLAND_DISPLAY"] = str(Path(os.environ["XDG_RUNTIME_DIR"]) / wayland)
            env.pop("AT_SPI_BUS_ADDRESS", None)
            config = Path(temporary) / "session.conf"
            config.write_text("""<busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen>
<policy context="default"><allow send_destination="*"/><allow receive_sender="*"/>
<allow own="*"/></policy></busconfig>""")
            result = subprocess.run(["dbus-run-session", "--config-file=" + str(config), "--", sys.executable,
                                     str(Path(__file__).resolve()), str(binary)]
                                    + (["--require-actions"] if args.require_actions else []), env=env)
            sys.exit(result.returncode)
