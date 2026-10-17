use freya::{code_editor::*, prelude::*};
use freya_testing::prelude::*;

fn editor_app() -> impl IntoElement {
    source_editor_app("")
}

fn source_editor_app(text: &'static str) -> impl IntoElement {
    use_init_theme(|| light_theme().with_light_code_editor());
    let editor = use_state(move || source_data(text));
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

fn shortcut(test: &mut TestingRunner, key: &str, code: Code, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character(key.into()),
        code,
        modifiers,
    });
    test.sync_and_update();
}

#[test]
fn source_background_highlights_follow_selection_replacement_undo_and_redo() {
    let mut test = launch_test(|| source_editor_app("# Original 🦊"));
    test.sync_and_update();
    shortcut(&mut test, "a", Code::KeyA, Modifiers::CONTROL);
    test.write_text("# Replaced é💬");
    assert_eq!(buffer(&test).as_deref(), Some("# Replaced é💬"));
    shortcut(&mut test, "z", Code::KeyZ, Modifiers::CONTROL);
    assert_eq!(buffer(&test).as_deref(), Some("# Original 🦊"));
    shortcut(&mut test, "y", Code::KeyY, Modifiers::CONTROL);
    assert_eq!(buffer(&test).as_deref(), Some("# Replaced é💬"));
    assert_colored_comment(&mut test, "Replaced é💬");
}

fn source_data(text: &str) -> CodeEditorData {
    let language = EditorLanguage::new(
        recite_writer_grammar::LANGUAGE,
        recite_writer_grammar::HIGHLIGHTS,
    );
    let mut editor = CodeEditorData::new(Rope::from_str(text), Some(language));
    assert!(editor.enable_background_highlighting().is_ok());
    editor.measure(20., "sans-serif");
    editor
}

fn source_lifecycle_app() -> impl IntoElement {
    use_init_theme(|| light_theme().with_light_code_editor());
    let mut editor = use_state(|| source_data("# First"));
    let mut visible = use_state(|| true);
    let focus = use_a11y();
    rect()
        .expanded()
        .maybe_child((*visible.read()).then(|| {
            CodeEditor::new(editor, focus)
                .a11y_auto_focus(true)
                .font_family("sans-serif")
                .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                    match &event.key {
                        Key::Named(NamedKey::F6) => editor.set(source_data("# Second 🦊")),
                        Key::Named(NamedKey::F7) => {
                            let mut data = CodeEditorData::new(Rope::from_str("Plain prose"), None);
                            data.parse();
                            data.measure(20., "sans-serif");
                            editor.set(data);
                        }
                        Key::Named(NamedKey::F8) => editor.set(source_data("# Third 💬")),
                        Key::Named(NamedKey::F9) => {
                            editor.set(source_data(&"# Pending source\n".repeat(50_000)));
                            editor.set(CodeEditorData::new(Rope::new(), None));
                            visible.set(false);
                        }
                        _ => return true,
                    }
                    false
                })
        }))
        .child(label().text(format!("visible={}", visible.read())))
}

fn assert_colored_comment(test: &mut TestingRunner, text: &str) {
    let comment = EditorSyntaxTheme::default().comment;
    for _ in 0..5_000 {
        test.poll_n(std::time::Duration::from_millis(16), 2);
        let colored = test
            .find(|_, element| {
                Paragraph::try_downcast(element).filter(|paragraph| {
                    paragraph.spans.iter().any(|span| {
                        span.text.contains(text)
                            && span.text_style_data.color == Some(comment.into())
                    })
                })
            })
            .is_some();
        if colored {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("current source colors never reached the rendered paragraph: {text}");
}

#[test]
fn source_notifications_rebind_across_replacement_prose_and_unmount() {
    let mut test = launch_test(source_lifecycle_app);
    test.sync_and_update();
    assert_colored_comment(&mut test, "First");
    test.press_key(Key::Named(NamedKey::F6));
    assert_colored_comment(&mut test, "Second 🦊");
    test.press_key(Key::Named(NamedKey::F7));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    test.press_key(Key::Named(NamedKey::F8));
    assert_colored_comment(&mut test, "Third 💬");
    test.press_key(Key::Named(NamedKey::F9));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == "visible=false"))
            .is_some()
    );
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element))
            .is_none()
    );
}

fn source_resize_app() -> impl IntoElement {
    let editor =
        use_state(|| source_data(&(0..80).map(|n| format!("# line {n}\n")).collect::<String>()));
    let mut height = use_state(|| 80.);
    let focus = use_a11y();
    rect().expanded().child(
        rect().height(Size::px(*height.read())).child(
            CodeEditor::new(editor, focus)
                .a11y_auto_focus(true)
                .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                    if event.key == Key::Named(NamedKey::F5) {
                        height.set(400.);
                        false
                    } else {
                        true
                    }
                }),
        ),
    )
}

#[test]
fn source_resize_highlights_new_visible_rows_without_scrolling() {
    let mut test = launch_test(source_resize_app);
    test.sync_and_update();
    assert_colored_comment(&mut test, "line 0");
    test.press_key(Key::Named(NamedKey::F5));
    assert_colored_comment(&mut test, "line 15");
}
