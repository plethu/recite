"""Reveal preference controls through acknowledged native scrolling, including after reflow."""

from native_window import key


def reveal_preference(window, label):
    coordinates = window.api.GLib.Variant("(u)", (int(window.api.Atspi.CoordType.WINDOW),))

    def home():
        scrollbar = window._outer_scrollbar()
        if scrollbar is None:
            return None
        viewport = window.call(scrollbar, "Component", "GetExtents", coordinates)[0]
        window.focus(scrollbar)
        key("Home")
        return viewport

    viewport = window.wait(home, "live preferences viewport")

    def within(node):
        rect = window.call(node, "Component", "GetExtents", coordinates)[0]
        return rect[1] >= viewport[1] and rect[1] + rect[3] <= viewport[1] + viewport[3]

    window.wait(
        lambda: (heading := window.find("Settings")) is not None and within(heading),
        "preferences heading after Home",
    )

    def positions():
        content = []
        for node, name in window.nodes():
            if (
                name
                and "org.a11y.atspi.Component"
                in window.call(node, "Accessible", "GetInterfaces")[0]
            ):
                rect = window.call(node, "Component", "GetExtents", coordinates)[0]
                if rect[1] >= viewport[1] and rect[1] < viewport[1] + viewport[3]:
                    content.append((tuple(node), tuple(rect)))
        return content

    def seek():
        target = window.find(label)
        if target is not None and within(target):
            return True
        direction = "Page_Down"
        if target is not None:
            rect = window.call(target, "Component", "GetExtents", coordinates)[0]
            direction = "Up" if rect[1] < viewport[1] else "Down"
        before = positions()
        scrollbar = window._outer_scrollbar()
        window.focus(scrollbar)
        key(direction)
        window.wait(
            lambda: positions() != before,
            "preference scroll movement toward " + label,
            timeout=1,
        )
        return False

    window.wait(seek, "preference control: " + label)
