//! Cross-screen quality contracts over the running native app.
mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
fn fill(
    test: &mut TestingRunner,
    placeholder: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(placeholder)))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("input {placeholder}"))?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text(text);
    test.poll_n(std::time::Duration::from_millis(16), 5);
    Ok(())
}
fn key(test: &mut TestingRunner, key: NamedKey) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(key),
        code: Code::Unidentified,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 5);
}
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(text)))
        .is_some()
}
#[test]
fn project_search_can_continue_past_one_hundred_and_clear_without_losing_input()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    let mut source = String::from(":: start\n");
    for index in 0..106 {
        source.push_str(&format!(
            "> line_{index}@{index:020}\n  Lantern number {index}.\n"
        ));
    }
    source.push_str("-> END\n");
    std::fs::write(dir.path().join("scene.recite"), source)?;
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1400., 1000.),
        |_| {},
        1.,
    )
    .0;
    fill(&mut test, "Project folder", &dir.path().to_string_lossy())?;
    support::click(&mut test, "Open project")?;
    for _ in 0..20 {
        if has(&test, "Project opened.") {
            break;
        }
        test.poll_n(std::time::Duration::from_millis(16), 5);
    }
    assert!(has(&test, "Project opened."));
    fill(&mut test, "Search project words", "Lantern")?;
    assert!(has(&test, "100 of 106 saved passages"));
    support::click(&mut test, "Show more results")?;
    assert!(has(&test, "106 of 106 saved passages"));
    support::click(&mut test, "Clear Search project words…")?;
    test.write_text("no-such-passage");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    assert!(has(&test, "No matching saved passages"));
    support::click(&mut test, "Clear Search project words…")?;
    test.write_text("Lantern");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    assert!(has(&test, "100 of 106 saved passages"));
    key(&mut test, NamedKey::ArrowDown);
    key(&mut test, NamedKey::Enter);
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Lantern number 0."))))
            .is_some()
    );
    Ok(())
}
#[test]
fn scene_search_has_empty_feedback_and_opens_a_match_from_the_keyboard()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 900.), |_| {}, 1.).0;
    fill(&mut test, "Find scene or beat…", "no such scene")?;
    assert!(has(&test, "No matching scenes or beats"));
    support::click(&mut test, "Clear Find scene or beat…")?;
    test.write_text("Last Tram");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    key(&mut test, NamedKey::ArrowDown);
    key(&mut test, NamedKey::Enter);
    support::open_beat(&mut test)?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("Only if we get on it."))))
            .is_some()
    );
    Ok(())
}

#[test]
fn destination_picker_search_changes_only_the_reply_and_supports_undo()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1400., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    assert!(has(&test, "▸ Missing Courier"));
    support::click(&mut test, "Change destination…")?;
    test.write_text("end");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    support::click(&mut test, "End conversation · END")?;
    assert!(!has(&test, "▸ Missing Courier"));
    assert!(has(&test, "▸ Station History"));
    support::click(&mut test, "Undo")?;
    assert!(has(&test, "▸ Missing Courier"));
    Ok(())
}

#[test]
fn diagnostic_opens_source_and_places_the_edit_cursor_at_its_location()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start\n> line@11111111111111111111\n  Hello.\n-> missing\n";
    let model = recite_writer_model::Workbench::new(source)?;
    let diagnostics = model.document().diagnostics();
    let diagnostic = diagnostics.first().ok_or("diagnostic")?;
    let span = &diagnostic.span;
    let caption = format!(
        "Open diagnostic {} at {}:{}:{}",
        diagnostic.code,
        span.file,
        span.start.line(),
        span.start.column()
    );
    let line = source
        .lines()
        .nth(span.start.line() as usize - 1)
        .ok_or("line")?;
    let column = span.start.column() as usize - 1;
    let expected = format!(
        "{}X{}",
        line.chars().take(column).collect::<String>(),
        line.chars().skip(column).collect::<String>()
    );
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1200., 900.),
        |runner| {
            runner.provide_root_context(|| recite_writer::InitialSource(source.into()));
        },
        1.,
    )
    .0;
    support::click(&mut test, "Source")?;
    support::click(&mut test, &caption)?;
    test.write_text("X");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .map(|s| s.text.as_ref())
            .collect::<String>()
            .contains(&expected)))
            .is_some(),
        "expected {expected}"
    );
    Ok(())
}
