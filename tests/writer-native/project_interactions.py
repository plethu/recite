"""Cross-document checks against the real native fields, including equal text."""

import time

from native_window import key, type_text


def manuscript(window):
    return window.wait(
        lambda: next(
            (
                node
                for node, name in window.nodes()
                if name == "Manuscript" and window.call(node, "Accessible", "GetRole")[0] == 79
            ),
            None,
        ),
        "current manuscript field",
    )


def document_caption(window, name, selected=None):
    key = name.removesuffix(" · Unsaved")
    for node, caption in window.nodes():
        clean = caption.removesuffix(" · Unsaved")
        current = clean.startswith("▾ ") or clean == "Current · " + key
        matches = clean in (key, "Current · " + key) or (
            clean.startswith(("▾ ", "▸ ")) and clean.endswith(" · " + key)
        )
        if (
            matches
            and (selected is None or current == selected)
            and "org.a11y.atspi.Action" in window.call(node, "Accessible", "GetInterfaces")[0]
        ):
            return caption
    return None


def switch(window, name, text):
    before = manuscript(window)
    caption = window.wait(lambda: document_caption(window, name), "document selector: " + name)
    assert window.act(caption), "document switch action was rejected"
    # Action delivery and AccessKit tree replacement are asynchronous. Resolve the
    # live field after switching instead of reusing the retired document's node.
    return window.wait(
        lambda: (
            document_caption(window, name, selected=True)
            and next(
                (
                    node
                    for node, label in window.nodes()
                    if label == "Manuscript"
                    and tuple(node) != tuple(before)
                    and window.call(node, "Accessible", "GetRole")[0] == 79
                    and window.text(node) == text
                ),
                None,
            )
        ),
        "retained document draft",
    )


def exercise(window, primary, results):
    before = manuscript(window)
    node = switch(window, "zz-other.recite", primary)
    assert tuple(node) != tuple(before), "equal text must still get a different native field"
    window.focus(node)
    key("End", control=True)
    type_text("x")
    window.wait(lambda: window.text(node) == primary + "x", "second-file typing")
    key("z", control=True)
    window.wait(lambda: window.text(node) == primary, "second-file undo")
    key("z", control=True)
    time.sleep(0.1)  # A no-op undo has no accessibility event to acknowledge it.
    assert window.text(node) == primary, "native undo must not reach into the previous file"
    key("y", control=True)
    window.wait(lambda: window.text(node) == primary + "x", "second-file redo")
    node = switch(window, "scene.recite · Unsaved", primary)
    results["document_switch_retains_both_drafts"] = True
    results["equal_text_fields_have_separate_native_history"] = True
    return node
