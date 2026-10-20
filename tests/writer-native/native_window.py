"""Keyboard and accessibility operations scoped to an owned native Writer window."""

import subprocess


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

    def wait(self, operation, label):
        return self.api.wait_for(operation, label, self.process, timeout=10)

    def act(self, label):
        node = self.find(label)
        if node is None:
            raise RuntimeError("Missing probe action: " + label)
        return self.call(node, "Action", "DoAction", self.api.GLib.Variant("(i)", (0,)))[0]

    def text(self, node):
        return self.call(node, "Text", "GetText", self.api.GLib.Variant("(ii)", (0, -1)))[0]

    def focus(self, node):
        self.call(node, "Component", "GrabFocus")
        bit = 1 << int(self.api.Atspi.StateType.FOCUSED)
        self.wait(lambda: self.call(node, "Accessible", "GetState")[0][0] & bit, "editor focus")


def key(name, control=False):
    arguments = ["wtype"]
    if control:
        arguments += ["-M", "ctrl"]
    arguments += ["-k", name]
    if control:
        arguments += ["-m", "ctrl"]
    subprocess.run(arguments, check=True, timeout=5, capture_output=True)


def type_text(text):
    subprocess.run(["wtype", "--", text], check=True, timeout=10, capture_output=True)
