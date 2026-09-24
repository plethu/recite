use super::*;
use freya::{code_editor::*, elements::paragraph::ParagraphCursorExt, text_edit::TextEditor};
use freya_testing::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app() -> impl IntoElement {
    let (mut editor, viewport) = use_consume::<(State<CodeEditorData>, EditorViewport)>();
    viewport.observe(editor, 14.);
    use_hook(move || editor.write().measure(14., "monospace"));
    CodeEditor::new(editor, use_a11y())
        .scroll_controller(viewport.scroll)
        .gutter(false)
        .font_family("monospace")
        .a11y_auto_focus(true)
}

fn source() -> String {
    (0..200)
        .map(|row| format!("row {row} {}🦀 café target", "x".repeat(200)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn setup(before_layout: bool) -> (TestingRunner, (State<CodeEditorData>, EditorViewport)) {
    TestingRunner::new(
        app,
        (500., 400.).into(),
        |runner| {
            runner.provide_root_context(|| {
                let mut editor = CodeEditorData::new(Rope::from_str(&source()), None);
                editor.parse();
                let viewport = EditorViewport {
                    scroll: ScrollController::new(0, 0),
                    reveal: State::create(before_layout),
                    completion: State::create(false),
                    candidate: State::create(None),
                };
                if before_layout {
                    let offset = editor.rope.char_to_utf16_cu(editor.rope.line_to_char(150)) + 210;
                    editor.move_cursor_to(offset);
                    viewport.request();
                }
                (State::create(editor), viewport)
            })
        },
        1.,
    )
}

fn assert_caret_visible(test: &TestingRunner) -> TestResult {
    let viewport = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|rect| {
                    rect.accessibility.builder.role() == AccessibilityRole::TextInput
                        && node.layout().area.height() > 100.
                })
                .map(|_| node.layout().area)
        })
        .ok_or("scroll viewport")?;
    let caret = test
        .find(|node, element| {
            let paragraph = Paragraph::try_downcast(element)?;
            let index = paragraph.cursor_index?;
            let text: String = paragraph
                .spans
                .iter()
                .map(|span| span.text.as_ref())
                .collect();
            let holder = paragraph.sk_paragraph.0.borrow();
            let measured = holder
                .as_ref()?
                .paragraph
                .cursor_rect(&text, index, TextAlign::Left);
            let area = node.layout().inner_area;
            Some((
                area.min_x() + measured.left,
                area.min_x() + measured.right.max(measured.left + 6.),
                area.min_y(),
                area.max_y(),
            ))
        })
        .ok_or("mounted caret paragraph")?;
    assert!(
        caret.0 >= viewport.min_x() && caret.1 <= viewport.max_x(),
        "horizontal {caret:?} in {viewport:?}"
    );
    assert!(
        caret.2 >= viewport.min_y() && caret.3 <= viewport.max_y(),
        "vertical {caret:?} in {viewport:?}"
    );
    Ok(())
}

#[test]
fn reveals_diagnostics_in_both_directions_without_changing_source() -> TestResult {
    let (mut test, (mut editor, viewport)) = setup(false);
    test.poll_n(std::time::Duration::from_millis(16), 8);
    for (row, column) in [(150, 215), (0, 0), (199, 210), (20, 0), (20, 0)] {
        let offset = {
            let data = editor.read();
            data.rope.char_to_utf16_cu(data.rope.line_to_char(row)) + column
        };
        {
            let mut data = editor.write();
            data.move_cursor_to(offset);
            viewport.request();
        }
        test.poll_n(std::time::Duration::from_millis(16), 8);
        assert_caret_visible(&test)?;
        assert_eq!(editor.read().cursor_pos(), offset);
        assert_eq!(editor.read().rope.to_string(), source());
    }
    Ok(())
}

#[test]
fn defers_reveal_until_first_layout() -> TestResult {
    let (mut test, _) = setup(true);
    test.poll_n(std::time::Duration::from_millis(16), 8);
    assert_caret_visible(&test)
}

#[test]
fn keyboard_movement_reveals_long_lines() -> TestResult {
    let (mut test, _) = setup(false);
    test.poll_n(std::time::Duration::from_millis(16), 8);
    test.press_key(Key::Named(NamedKey::End));
    assert_caret_visible(&test)?;
    test.press_key(Key::Named(NamedKey::Home));
    assert_caret_visible(&test)
}

#[test]
fn manual_scrolling_stays_put_until_an_explicit_reveal() -> TestResult {
    let (mut test, (_editor, viewport)) = setup(false);
    test.poll_n(std::time::Duration::from_millis(16), 8);
    test.scroll((250., 200.), (-600., -1800.));
    test.poll_n(std::time::Duration::from_millis(16), 8);
    assert!(
        test.find(|_, element| {
            Paragraph::try_downcast(element).filter(|paragraph| paragraph.cursor_index.is_some())
        })
        .is_none(),
        "manual scroll should be able to leave the caret behind"
    );
    viewport.request();
    test.poll_n(std::time::Duration::from_millis(16), 8);
    assert_caret_visible(&test)
}
