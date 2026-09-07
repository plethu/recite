### Description

`CodeEditor` does not display IME composition text when it receives a preedit event. Multiline `Input` displays the same event without changing its bound value.

This was reproduced with `freya-testing`, not a desktop IME. Committed Unicode text and line breaks work in the CodeEditor test.

### Steps to Reproduce

Use these dependencies in a Rust project:

```toml
[dev-dependencies]
freya = { version = "=0.5.0-rc.4", features = ["code-editor"] }
freya-testing = "=0.5.0-rc.4"
```

Save this as `tests/preedit.rs` and run `cargo test --test preedit`:

```rust
use freya::{code_editor::*, prelude::*};
use freya_testing::prelude::*;

fn app() -> impl IntoElement {
    use_init_theme(|| light_theme().with_light_code_editor());
    let editor = use_state(|| {
        let mut editor = CodeEditorData::new(Rope::new(), None);
        editor.parse();
        editor.measure(20., "sans-serif");
        editor
    });
    let focus = use_a11y();
    rect().expanded().child(
        rect().height(Size::px(180.)).child(
            CodeEditor::new(editor, focus)
                .a11y_auto_focus(true)
                .gutter(false)
                .font_size(20.)
                .font_family("sans-serif"),
        ),
    )
}

#[test]
fn composition_is_visible() {
    let mut test = launch_test(app);
    test.sync_and_update(); // settle autofocus before sending input
    test.send_event(PlatformEvent::ImePreedit {
        name: ImeEventName::Preedit,
        text: "にほん".into(),
        cursor: Some((0, 9)),
    });
    test.sync_and_update();
    assert!(test.find(|_, element| {
        Paragraph::try_downcast(element).filter(|paragraph| {
            paragraph.spans.iter().any(|span| span.text.contains("にほん"))
        })
    }).is_some());
}
```

The test compiles, then fails at the assertion: no rendered paragraph contains `にほん`.

### Expected Behavior

Display the composition text while keeping it out of the document until the IME commits it. Cancelling composition should remove it.

### Freya Version

0.5.0-rc.4 (crates.io)

### Rust Version

rustc 1.96.0 (ac68faa20 2026-05-25)

### OS Version

CachyOS Linux (rolling), x86_64. Headless test runner.

### Additional Context

The equivalent test with `Input::new(value).multiline(true).auto_focus(true)` displays the composition text and leaves `value` unchanged. Clearing preedit and supplying committed text updates the value.

Main at `bf825f733fdebc26963e4a1f62621ff4816a1286` also has no preedit handler in [editor_ui.rs](https://github.com/marc2332/freya/blob/bf825f733fdebc26963e4a1f62621ff4816a1286/crates/freya-code-editor/src/editor_ui.rs). Input has an `on_ime_preedit` handler and renders preedit segments. That main revision was inspected but not built.
