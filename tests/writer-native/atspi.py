"""Raw AT-SPI D-Bus helpers shared by the owned native Writer scenarios."""

import subprocess
import time
from contextlib import contextmanager
from importlib import import_module

import gi

gi.require_version("Gio", "2.0")
gi.require_version("Atspi", "2.0")
Atspi = import_module("gi.repository.Atspi")
Gio = import_module("gi.repository.Gio")
GLib = import_module("gi.repository.GLib")

ACCESSIBLE = "org.a11y.atspi.Accessible"
ROOT = "/org/a11y/atspi/accessible/root"


def call(bus, address, interface, method, arguments=None, *, flags=Gio.DBusCallFlags.NONE):
    return bus.call_sync(
        address[0],
        address[1],
        interface,
        method,
        arguments,
        None,
        flags,
        2000,
        None,
    ).unpack()


def property_value(bus, address, interface, name):
    return call(
        bus,
        address,
        "org.freedesktop.DBus.Properties",
        "Get",
        GLib.Variant("(ss)", (interface, name)),
    )[0]


def wait_for(operation, description, process, timeout=20):
    deadline = time.monotonic() + timeout
    last_error = None
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Child process exited with {process.returncode}: {description}")
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
            raise RuntimeError("Accessibility tree exceeded the traversal deadline")
        address = tuple(pending.pop())
        if address in seen:
            continue
        seen.add(address)
        if len(seen) > 5000:
            raise RuntimeError("Accessibility tree exceeds the probe bound")
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
