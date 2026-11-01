"""Require the actual text range to fit inside its owned native viewport."""

from native_window import key


def reveal_text(window, value):
    coord = int(window.api.Atspi.CoordType.WINDOW)
    coordinates = window.api.GLib.Variant("(u)", (coord,))

    def visible():
        node = window.find_text(value, contains=True)
        viewport = window.content_scrollbar(value)
        if node is None:
            if viewport is None:
                return False
            window.focus(viewport)
            key("Page_Down")
            return False
        interfaces = window.call(node, "Accessible", "GetInterfaces")[0]
        if "org.a11y.atspi.Text" in interfaces:
            text = window.text(node)
            start = text.index(value)
            bounds = window.call(
                node,
                "Text",
                "GetRangeExtents",
                window.api.GLib.Variant("(iiu)", (start, start + len(value), coord)),
            )
        else:
            bounds = window.call(node, "Component", "GetExtents", coordinates)[0]
        frame = viewport or next(
            address
            for address, _ in window.nodes()
            if window.call(address, "Accessible", "GetRole")[0]
            in (int(window.api.Atspi.Role.FRAME), int(window.api.Atspi.Role.WINDOW))
        )
        clip = window.call(frame, "Component", "GetExtents", coordinates)[0]
        assert bounds[2] > 0 and bounds[3] > 0, f"Invalid native text bounds: {bounds}"
        if bounds[1] >= clip[1] and bounds[1] + bounds[3] <= clip[1] + clip[3]:
            return True
        assert viewport is not None, "Clipped text has no native scrolling viewport"
        window.focus(viewport)
        key("Up" if bounds[1] < clip[1] else "Down")
        return False

    window.wait(visible, "complete visible text range: " + value)
