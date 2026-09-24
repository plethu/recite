mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: key.clone(),
        code,
        modifiers,
    });
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyUp,
        key,
        code,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(Duration::from_millis(16), 8);
}
fn colon(test: &mut TestingRunner) {
    key(
        test,
        Key::Character(":".into()),
        Code::Semicolon,
        Modifiers::SHIFT,
    );
}
fn palette(test: &TestingRunner) -> bool {
    test.find(|_, e| {
        Rect::try_downcast(e).filter(|r| r.accessibility.builder.label() == Some("Close commands"))
    })
    .is_some()
}
fn vim(test: &mut TestingRunner) -> Result {
    support::click(test, "Settings")?;
    support::click(test, "Keymap: Vim")?;
    support::click(test, "Done")
}
#[test]
fn colon_opens_commands_only_with_vim_navigation() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Map")?;
    colon(&mut test);
    assert!(!palette(&test));
    vim(&mut test)?;
    support::click(&mut test, "Map")?;
    colon(&mut test);
    assert!(palette(&test));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!palette(&test));
    // Logical colon also works on layouts where Shift is unnecessary.
    key(
        &mut test,
        Key::Character(":".into()),
        Code::Semicolon,
        Modifiers::empty(),
    );
    assert!(palette(&test));
    Ok(())
}
#[test]
fn search_insertion_keeps_colons_and_normal_mode_opens_commands() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    vim(&mut test)?;
    support::click(&mut test, "Find scene or beat…")?;
    colon(&mut test);
    assert!(!palette(&test));
    assert!(
        test.find(
            |_, e| Rect::try_downcast(e).filter(|r| r.accessibility.builder.role()
                == AccessibilityRole::TextInput
                && r.accessibility.builder.label() == Some(":"))
        )
        .is_some()
    );
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    colon(&mut test);
    assert!(palette(&test));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Character("i".into()),
        Code::KeyI,
        Modifiers::empty(),
    );
    colon(&mut test);
    assert!(!palette(&test));
    Ok(())
}
#[test]
fn source_entry_keeps_colon_and_settings_do_not_open_commands() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    vim(&mut test)?;
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Source editor")?;
    colon(&mut test);
    assert!(!palette(&test));
    support::click(&mut test, "Settings")?;
    colon(&mut test);
    assert!(!palette(&test));
    Ok(())
}

#[test]
fn prose_entry_in_vim_mode_inserts_a_literal_colon() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    vim(&mut test)?;
    let prose = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.starts_with("If you're here")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(prose.min_x() + 3.), f64::from(prose.min_y() + 3.)));
    test.poll_n(Duration::from_millis(16), 8);
    colon(&mut test);
    assert!(!palette(&test));
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| {
            let text: String = p.spans.iter().map(|s| s.text.as_ref()).collect();
            text.contains("If you're here") && text.contains(':')
        }))
        .is_some()
    );
    Ok(())
}

fn text(test: &TestingRunner, expected: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(expected)))
        .is_some()
}
#[test]
fn pane_sequence_moves_focus_and_escape_cancels() -> Result {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1400., 1000.).into(),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    vim(&mut test)?;
    support::click(&mut test, "Beat editor")?;
    key(
        &mut test,
        Key::Character("w".into()),
        Code::KeyW,
        Modifiers::CONTROL,
    );
    assert!(text(&test, "Ctrl+w →"));
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("vim-panes.png"));
    }
    key(
        &mut test,
        Key::Character("h".into()),
        Code::KeyH,
        Modifiers::empty(),
    );
    assert!(!text(&test, "Ctrl+w →"));
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Scene navigation")
    );
    key(
        &mut test,
        Key::Character("w".into()),
        Code::KeyW,
        Modifiers::CONTROL,
    );
    key(
        &mut test,
        Key::Character("l".into()),
        Code::KeyL,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Beat editor")
    );
    key(
        &mut test,
        Key::Character("w".into()),
        Code::KeyW,
        Modifiers::CONTROL,
    );
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!text(&test, "Ctrl+w →"));
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Beat editor")
    );
    Ok(())
}
#[test]
fn slash_and_repeat_search_preserve_the_query() -> Result {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1400., 1000.).into(),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    vim(&mut test)?;
    support::click(&mut test, "Beat editor")?;
    key(
        &mut test,
        Key::Character("/".into()),
        Code::Slash,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().role(),
        AccessibilityRole::TextInput
    );
    test.write_text("courier");
    test.poll_n(Duration::from_millis(16), 8);
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Character("n".into()),
        Code::KeyN,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Beat editor")
    );
    let chosen = |test: &TestingRunner, name: &str| {
        test.find(|_, e| {
            Rect::try_downcast(e).filter(|r| {
                r.accessibility.builder.label() == Some(name)
                    && r.accessibility.builder.toggled() == Some(accesskit::Toggled::True)
            })
        })
        .is_some()
    };
    assert!(chosen(&test, "Missing Courier"));
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("vim-search.png"));
    }
    key(
        &mut test,
        Key::Character("n".into()),
        Code::KeyN,
        Modifiers::empty(),
    );
    assert!(chosen(&test, "Courier Route"));
    key(
        &mut test,
        Key::Character("N".into()),
        Code::KeyN,
        Modifiers::SHIFT,
    );
    assert!(chosen(&test, "Missing Courier"));
    key(
        &mut test,
        Key::Character("/".into()),
        Code::Slash,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("courier")
    );
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("courierj")
    );
    Ok(())
}
#[test]
fn history_chords_follow_the_existing_back_and_forward_transitions() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1400., 1000.).into(), |_| {}, 1.).0;
    vim(&mut test)?;
    support::click(&mut test, "Localize")?;
    assert!(text(&test, "Localisation is optional"));
    key(
        &mut test,
        Key::Character("o".into()),
        Code::KeyO,
        Modifiers::CONTROL,
    );
    assert!(!text(&test, "Localisation is optional"));
    key(
        &mut test,
        Key::Character("i".into()),
        Code::KeyI,
        Modifiers::CONTROL,
    );
    assert!(text(&test, "Localisation is optional"));
    Ok(())
}
#[test]
fn quit_alias_uses_the_existing_confirmation() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    vim(&mut test)?;
    colon(&mut test);
    test.write_text("q");
    test.poll_n(Duration::from_millis(16), 8);
    assert!(text(&test, "Close application"));
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(!palette(&test));
    assert!(text(&test, "Close Recite"));
    support::click(&mut test, "Close Recite?")?;
    colon(&mut test);
    assert!(!palette(&test));
    Ok(())
}
