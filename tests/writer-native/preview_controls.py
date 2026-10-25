"""Preview submission remains reachable without scrolling its conversation or inputs."""


def submission(window):
    caption = window.wait(
        lambda: next(
            (name for name in ("Start preview", "Restart preview") if window.find(name)), None
        ),
        "idle preview submission",
    )
    control = window.find(caption)
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))
    frame = window.call(window.application, "Accessible", "GetChildren")[0][0]
    bounds = window.call(control, "Component", "GetExtents", coordinates)[0]
    viewport = window.call(frame, "Component", "GetExtents", coordinates)[0]
    assert bounds[0] >= viewport[0] and bounds[1] >= viewport[1]
    assert bounds[0] + bounds[2] <= viewport[0] + viewport[2]
    assert bounds[1] + bounds[3] <= viewport[1] + viewport[3]
    return caption


def start(window):
    caption = submission(window)
    assert window.enabled(caption)
    window.act(caption)


def diagnostic_focus(window):
    focused = 1 << int(window.api.Atspi.StateType.FOCUSED)

    def ready():
        node = window.find_action("Hide preview error")
        return node is not None and window.call(node, "Accessible", "GetState")[0][0] & focused

    return window.wait(
        ready,
        "error disclosure keeps its own focus anchor",
    )


def focused_in_content(window, node):
    focused = 1 << int(window.api.Atspi.StateType.FOCUSED)
    if node is None or not window.call(node, "Accessible", "GetState")[0][0] & focused:
        return False
    return within_content(window, node)


def within_content(window, node):
    if node is None:
        return False
    scrollbar = window.section_scrollbar("Conversation preview")
    if scrollbar is None:
        return False
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))
    bounds = window.call(node, "Component", "GetExtents", coordinates)[0]
    clip = window.call(scrollbar, "Component", "GetExtents", coordinates)[0]
    return bounds[1] >= clip[1] and bounds[1] + bounds[3] <= clip[1] + clip[3]
