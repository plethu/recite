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
        self.document_action("Close")
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
            return (
                value in text if contains else (text.startswith(value) if prefix else text == value)
            )

        for node, name in self.nodes():
            if matches(name):
                return node
            if "org.a11y.atspi.Text" in self.call(node, "Accessible", "GetInterfaces")[0]:
                text = self.text(node)
                if matches(text):
                    return node
        return None

    def act(self, label):
        def activate():
            node = self.find_action(label)
            if node is None:
                return False
            enabled = 1 << int(self.api.Atspi.StateType.ENABLED)
            if not self.call(node, "Accessible", "GetState")[0][0] & enabled:
                return False
            return self.call(node, "Action", "DoAction", self.api.GLib.Variant("(i)", (0,)))[0]

        return self.wait(activate, "enabled action: " + label)

    def find_action(self, label):
        return next(
            (
                node
                for node, name in self.nodes()
                if name == label
                and "org.a11y.atspi.Action" in self.call(node, "Accessible", "GetInterfaces")[0]
            ),
            None,
        )

    def document_control(self, label):
        """Follow the document menu's named native group and its own viewport."""

        def open_menu():
            if self.find_action(label) is None and self.find("Document actions menu") is None:
                self.act("Workspace")
            return True

        self.wait(open_menu, "document action or open menu: " + label)

        def page():
            if self.find_action(label) is not None:
                return True
            scrollbar = self.section_scrollbar("Document actions menu")
            if scrollbar is not None:
                self.focus(scrollbar)
                key("Page_Down")
            return False

        self.wait(page, "document menu action: " + label)
        return self.find_action(label)

    def document_action(self, label):
        self.document_control(label)
        return self.act(label)

    def workspace_action(self, label):
        """Reach app actions through the compact workbench's Workspace menu."""

        def open_menu():
            if self.find_action(label) is None:
                self.act("Workspace")
            return True

        self.wait(open_menu, "workspace action or menu: " + label)
        return self.act(label)

    def section_scrollbar(self, label):
        """Resolve a named pane's own ScrollView through standard AT-SPI children."""
        for anchor, name in self.nodes():
            if name != label:
                continue
            pending, seen = [anchor], set()
            while pending:
                node = pending.pop()
                if tuple(node) in seen:
                    continue
                seen.add(tuple(node))
                children = self.call(node, "Accessible", "GetChildren")[0]
                vertical = 1 << int(self.api.Atspi.StateType.VERTICAL)
                scrollbar = next(
                    (
                        child
                        for child in children
                        if self.call(child, "Accessible", "GetRole")[0]
                        == int(self.api.Atspi.Role.SCROLL_BAR)
                        and self.call(child, "Accessible", "GetState")[0][0] & vertical
                    ),
                    None,
                )
                if scrollbar is not None:
                    return scrollbar
                pending.extend(children)
        return None

    def content_scrollbar(self, label):
        if label == "Manuscript" or label.startswith("Edit passage ") or label == "Pin reference":
            return self.section_scrollbar("Script pane") or self._outer_scrollbar()
        return self._outer_scrollbar()

    def edit_passage(self, ordinal):
        node = self.wait(lambda: self.find(f"Edit passage {ordinal}"), "inline manuscript field")
        self.focus(node)
        self.wait(lambda: self.find("Manuscript") == node, "admitted inline manuscript")

    def reveal_down(self, label, *, prefix=False, contains=False):
        """Reach a content action by paging its owned native document viewport."""

        def page():
            if self.find_text(label, prefix=prefix, contains=contains):
                return True
            scrollbar = self.content_scrollbar(label)
            if scrollbar is None:
                return False
            self.focus(scrollbar)
            key("Page_Down")
            return False

        self.wait(page, "paged content action: " + label)

    def reveal_end(self, label):
        """Reach a content action through its named native viewport."""
        self._reveal(label)

    def reveal_start(self, label, *, prefix=False, contains=False):
        """Reach a content action through its named native viewport."""
        self._reveal(label, prefix=prefix, contains=contains)

    def _reveal(self, label, *, prefix=False, contains=False):
        def scroll():
            if self.find_text(label, prefix=prefix, contains=contains):
                return True
            scrollbar = self.content_scrollbar(label)
            if scrollbar is None:
                return False
            self.focus(scrollbar)
            key("Home")
            return True

        # A pane transition can remove AT-SPI nodes during traversal. Retry the
        # complete live lookup, as act()/field lookup already do.
        self.wait(scroll, "native scroll toward: " + label)
        self.reveal_down(label, prefix=prefix, contains=contains)

    def _outer_scrollbar(self):
        # Named panes remain exposed while their content scrolls. Prefer the
        # active preview for content probes; manuscript requests select Script.
        for label in ("Conversation preview", "Script pane"):
            scrollbar = self.section_scrollbar(label)
            if scrollbar is not None:
                return scrollbar
        vertical = 1 << int(self.api.Atspi.StateType.VERTICAL)
        scrollbars = [
            node
            for node, _ in self.nodes()
            if self.call(node, "Accessible", "GetRole")[0] == 48
            and self.call(node, "Accessible", "GetState")[0][0] & vertical
        ]
        coordinates = self.api.GLib.Variant("(u)", (int(self.api.Atspi.CoordType.WINDOW),))
        # The writing rail scrolls independently. The content portal reaches the
        # right window edge; height alone would select the rail or a pinned preview.
        return max(
            scrollbars,
            key=lambda node: (
                self.call(node, "Component", "GetExtents", coordinates)[0][0],
                self.call(node, "Component", "GetExtents", coordinates)[0][3],
            ),
            default=None,
        )

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
    modifiers = [
        name for name, active in [("ctrl", control), ("shift", shift), ("alt", alt)] if active
    ]
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
                "org.freedesktop.DBus",
                "GetConnectionUnixProcessID",
                api.GLib.Variant("(s)", (child[0],)),
            )[0]
            if pid == process.pid:
                return child
        return None

    app = api.wait_for(application, "owned Writer registration", process)
    window = NativeWindow(api, bus, process, lambda: list(api.descendants(bus, app)))
    window.application = app
    return window
