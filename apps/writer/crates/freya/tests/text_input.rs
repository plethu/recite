use freya::{code_editor::*, prelude::*};
use freya_testing::prelude::*;

fn editor_app() -> impl IntoElement {
    use_init_theme(|| light_theme().with_light_code_editor());
    let editor = use_state(|| {
        let mut editor = CodeEditorData::new(Rope::new(), None);
        editor.parse();
        editor.measure(20., "sans-serif");
        editor
    });
    let focus = use_a11y();
    let text = editor.read().rope.to_string();
    rect()
        .expanded()
        .child(
            rect().height(Size::px(180.)).child(
                CodeEditor::new(editor, focus)
                    .a11y_auto_focus(true)
                    .gutter(false)
                    .font_size(20.)
                    .font_family("sans-serif"),
            ),
        )
        .child(label().text(format!("buffer={text}")))
}

fn buffer(test: &TestingRunner) -> Option<String> {
    test.find(|_, element| {
        Label::try_downcast(element)
            .and_then(|label| label.text.strip_prefix("buffer=").map(str::to_owned))
    })
}

fn input_app() -> impl IntoElement {
    let value = use_state(String::new);
    let mut continued = use_state(|| false);
    let text = value.read().clone();
    rect()
        .expanded()
        .child(
            Input::new(value)
                .multiline(true)
                .auto_focus(true)
                .width(Size::px(220.))
                .height(Size::px(180.)),
        )
        .child(label().text(format!("buffer={text}")))
        .child(
            Button::new()
                .on_press(move |_| continued.set(true))
                .child("Continue"),
        )
        .child(label().text(format!("continued={}", continued.read())))
}

#[test]
fn multiline_input_wraps_prose_and_tab_reaches_the_next_control()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = launch_test(input_app);
    test.sync_and_update();
    let prose = "A long question should wrap naturally while the writer keeps thinking about the conversation, without inserting line breaks to fit the window.";
    test.write_text(prose);
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|paragraph| paragraph.spans.iter().any(|span| span.text == prose))
                .map(|_| node.layout().area)
        })
        .ok_or("prose paragraph missing")?;
    assert!(
        area.height() > 40.,
        "long prose should occupy several visual lines"
    );
    assert_eq!(buffer(&test).as_deref(), Some(prose));
    test.press_key(Key::Named(NamedKey::Tab));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    test.press_key(Key::Named(NamedKey::Enter));
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == "continued=true"))
            .is_some()
    );
    assert_eq!(buffer(&test).as_deref(), Some(prose));
    Ok(())
}

#[test]
fn multiline_input_presents_preedit_without_committing_it() {
    let mut test = launch_test(input_app);
    test.sync_and_update();
    test.send_event(PlatformEvent::ImePreedit {
        name: ImeEventName::Preedit,
        text: "にほん".into(),
        cursor: Some((0, 9)),
    });
    test.sync_and_update();
    assert_eq!(buffer(&test).as_deref(), Some(""));
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text.contains("にほん")))
        )
        .is_some()
    );
    test.send_event(PlatformEvent::ImePreedit {
        name: ImeEventName::Preedit,
        text: String::new(),
        cursor: None,
    });
    test.sync_and_update();
    test.write_text("日本");
    test.press_key(Key::Named(NamedKey::Enter));
    test.write_text("Café 💬");
    assert_eq!(buffer(&test).as_deref(), Some("日本\nCafé 💬"));
}

#[test]
fn committed_unicode_and_hard_newlines_reach_the_buffer() {
    let mut test = launch_test(editor_app);
    test.sync_and_update();
    test.write_text("Café 💬");
    test.press_key(Key::Named(NamedKey::Enter));
    test.write_text("مرحبا");
    assert_eq!(buffer(&test).as_deref(), Some("Café 💬\nمرحبا"));
    test.press_key(Key::Named(NamedKey::Backspace));
    assert_eq!(buffer(&test).as_deref(), Some("Café 💬\nمرحب"));
}

#[test]
fn probe_records_missing_code_editor_preedit_presentation() {
    let mut test = launch_test(editor_app);
    test.sync_and_update();
    test.send_event(PlatformEvent::ImePreedit {
        name: ImeEventName::Preedit,
        text: "にほん".into(),
        cursor: Some((0, 9)),
    });
    test.sync_and_update();
    // This is a negative capability probe, not an IME acceptance test.
    assert_eq!(buffer(&test).as_deref(), Some(""));
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text.contains("にほん")))
        )
        .is_none()
    );
}

#[test]
fn prose_keyboard_undo_restores_the_previous_text() {
    let mut test = launch_test(input_app);
    test.sync_and_update();
    test.write_text("A different question.");
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("z".into()),
        code: Code::KeyZ,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    assert_eq!(buffer(&test).as_deref(), Some(""));
}
