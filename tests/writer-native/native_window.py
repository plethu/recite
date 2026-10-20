"""Keyboard and accessibility operations scoped to an owned native Writer window."""

import subprocess
import time
from collections import Counter
from pathlib import Path


class NativeWindow:
    def __init__(self, api, bus, process, nodes):
        self.api, self.bus, self.process, self.nodes = api, bus, process, nodes

    def call(self, node, interface, method, arguments=None):
        return self.api.call(self.bus, node, "org.a11y.atspi." + interface, method, arguments)

    def find(self, label, prefix=False):
        return next(
            (
                node
                for node, name in self.nodes()
                if (name.startswith(label) if prefix else name == label)
            ),
            None,
        )

    def wait(self, operation, label, timeout=10):
        return self.api.wait_for(operation, label, self.process, timeout=timeout)

    def exposure_diagnostic(self):
        """Report broken advertised edges; this never substitutes for a passing traversal."""
        pending = [(None, self.application)]
        seen, failures = set(), []
        while pending and len(seen) < 5000:
            parent, node = pending.pop()
            identity = tuple(node)
            if identity in seen:
                continue
            seen.add(identity)
            try:
                children = self.call(node, "Accessible", "GetChildren")[0]
            except self.api.GLib.Error as error:
                failures.append({"parent": parent, "advertised_child": node, "error": str(error)})
                continue
            pending.extend((node, child) for child in children)
        return {"visited": len(seen), "broken_edges": failures}

    def close_clean(self):
        self.act("Close")
        try:
            self.wait(lambda: self.find("Close Writer"), "ordinary exit preference")
        except RuntimeError:
            if self.process.poll() != 0:
                raise
        if self.process.poll() is None:
            self.act("Close Writer")

    def save_and_close(self, results, label):
        started = time.monotonic()
        assert self.act("Save all and close")
        try:
            code = self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            tasks = Path(f"/proc/{self.process.pid}/task")
            waits = Counter()
            for task in tasks.iterdir():
                try:
                    waits[task.joinpath("wchan").read_text().strip()] += 1
                except OSError:
                    pass  # A thread may exit while collecting this optional timing diagnostic.
            results[label + "_waits_at_five_seconds"] = dict(waits)
            code = self.process.wait(timeout=25)
        results[label + "_seconds"] = time.monotonic() - started
        assert code == 0

    def find_text(self, value, prefix=False, contains=False):
        def matches(text):
            return value in text if contains else (text.startswith(value) if prefix else text == value)

        for node, name in self.nodes():
            if matches(name):
                return node
            if "org.a11y.atspi.Text" in self.call(node, "Accessible", "GetInterfaces")[0]:
                text = self.text(node)
                if matches(text):
                    return node
        return None

    def act(self, label):
        node = self.wait(lambda: self.find(label), "available action: " + label)
        return self.call(node, "Action", "DoAction", self.api.GLib.Variant("(i)", (0,)))[0]

    def enabled(self, label):
        node = self.find(label)
        if node is None:
            raise RuntimeError("Missing probe control: " + label)
        state = self.call(node, "Accessible", "GetState")[0][0]
        enabled = bool(state & (1 << int(self.api.Atspi.StateType.ENABLED)))
        sensitive = bool(state & (1 << int(self.api.Atspi.StateType.SENSITIVE)))
        assert enabled == sensitive, (label, enabled, sensitive)
        return enabled

    def text(self, node):
        return self.call(node, "Text", "GetText", self.api.GLib.Variant("(ii)", (0, -1)))[0]

    def focus(self, node):
        self.call(node, "Component", "GrabFocus")
        bit = 1 << int(self.api.Atspi.StateType.FOCUSED)
        self.wait(lambda: self.call(node, "Accessible", "GetState")[0][0] & bit, "editor focus")


def key(name, control=False, shift=False, alt=False):
    arguments = ["wtype"]
    modifiers = [name for name, active in [("ctrl", control), ("shift", shift), ("alt", alt)] if active]
    for modifier in modifiers:
        arguments += ["-M", modifier]
    arguments += ["-k", name]
    for modifier in reversed(modifiers):
        arguments += ["-m", modifier]
    subprocess.run(arguments, check=True, timeout=5, capture_output=True)


def type_text(text):
    subprocess.run(["wtype", "--", text], check=True, timeout=10, capture_output=True)


def attach(api, bus, registry, process):
    """Find only the accessibility application owned by the supplied process."""
    def application():
        for child in api.call(bus, registry, api.ACCESSIBLE, "GetChildren")[0]:
            pid = api.call(
                bus,
                ("org.freedesktop.DBus", "/org/freedesktop/DBus"),
                "org.freedesktop.DBus", "GetConnectionUnixProcessID",
                api.GLib.Variant("(s)", (child[0],)),
            )[0]
            if pid == process.pid:
                return child
        return None

    app = api.wait_for(application, "owned Writer registration", process)
    window = NativeWindow(api, bus, process, lambda: list(api.descendants(bus, app)))
    window.application = app
    return window
