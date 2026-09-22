//! Native picker and dialog shortcuts, including text-entry boundaries.
mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
    for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
        test.send_event(PlatformEvent::Keyboard {
            name,
            key: key.clone(),
            code,
            modifiers,
        });
        test.poll_n(std::time::Duration::from_millis(16), 3);
    }
}
fn submit(test: &mut TestingRunner) {
    key(
        test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    );
}
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|label| label.text.contains(text)))
        .is_some()
}
fn button_area(test: &TestingRunner, text: &str) -> Result<Area, Box<dyn std::error::Error>> {
    test.find(|node, e| {
        Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some(text))
            .map(|_| node.layout().area)
    })
    .ok_or_else(|| format!("Missing button {text}").into())
}
fn start(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    support::open_beat(test)?;
    support::click(test, "Localize")?;
    support::click(test, "Start localisation")
}

#[test]
fn picker_keeps_dialog_still_and_allows_pointer_selection() -> Result<(), Box<dyn std::error::Error>>
{
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(900., 650.), |_| {}, 1.).0;
    support::dark_theme(&mut test)?;
    start(&mut test)?;
    let before = button_area(&test, "Create catalogue")?;
    submit(&mut test);
    assert!(has(&test, "Choose a language to see"));
    support::click(&mut test, "Choose language")?;
    test.write_text("French");
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_eq!(before, button_area(&test, "Create catalogue")?);
    if let Ok(path) = std::env::var("RECITE_PICKER_SCREENSHOT") {
        test.render_to_file(path);
    }
    submit(&mut test);
    assert!(has(&test, "fr-CA")); // Search submission cannot create a catalogue.
    support::click(&mut test, "French (Canada) · fr-CA")?;
    assert!(has(&test, "locale/fr-CA.po"));
    support::click(&mut test, "Cancel")?;
    Ok(())
}

#[test]
fn vim_picker_preserves_insert_text_and_navigates_in_normal_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(900., 650.), |_| {}, 1.).0;
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keymap: Vim")?;
    submit(&mut test);
    assert!(!has(&test, "Done"));
    start(&mut test)?;
    support::click(&mut test, "Choose language")?;
    test.write_text("jkbzz");
    test.poll_n(std::time::Duration::from_millis(16), 4);
    if let Ok(path) = std::env::var("RECITE_VIM_SCREENSHOT") {
        test.render_to_file(path);
    }
    assert!(has(&test, "No matching"));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    // First Escape leaves INSERT; second dismisses the picker, not the dialog.
    assert!(has(&test, "No matching"));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!has(&test, "No matching"));
    assert!(has(&test, "Create catalogue"));
    support::click(&mut test, "Choose language")?;
    test.write_text("French");
    test.poll_n(std::time::Duration::from_millis(16), 4);
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(has(&test, "NORMAL"));
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Character("k".into()),
        Code::KeyK,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has(&test, "locale/fr-CA.po"));
    support::click(&mut test, "Cancel")?;
    Ok(())
}

#[test]
fn keyboard_reaches_results_beyond_eight_and_tab_leaves_the_picker()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(900., 650.),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    start(&mut test)?;
    support::click(&mut test, "Choose language")?;
    test.write_text("a");
    test.poll_n(std::time::Duration::from_millis(16), 4);
    for _ in 0..12 {
        key(
            &mut test,
            Key::Named(NamedKey::ArrowDown),
            Code::ArrowDown,
            Modifiers::empty(),
        );
    }
    let (caption, area) = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility.builder.role() == AccessibilityRole::ListBoxOption
                        && r.accessibility.builder.is_selected() == Some(true)
                })
                .and_then(|r| {
                    r.accessibility
                        .builder
                        .label()
                        .map(|name| (name.to_owned(), node.layout().area))
                })
        })
        .ok_or("selected result after scrolling")?;
    assert!(area.min_y() >= 0. && area.max_y() <= 650.);
    let (_, tag) = caption.rsplit_once(" · ").ok_or("option locale code")?;
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has(&test, &format!("locale/{tag}.po")));
    support::click(&mut test, "Choose language")?;
    key(
        &mut test,
        Key::Named(NamedKey::Tab),
        Code::Tab,
        Modifiers::empty(),
    );
    let focused = platform.focused_accessibility_node.peek();
    assert_eq!(focused.label(), Some("Cancel"));
    Ok(())
}

#[test]
fn vim_navigation_is_shared_by_settings_options_and_passage_menus()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1000., 800.),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keymap: Vim")?;
    support::click(&mut test, "Theme: Light")?;
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Dark")
    );
    submit(&mut test);
    support::open_beat(&mut test)?;
    let field = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|span| span.text.contains("If you're here"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(field.min_x() + 4.), f64::from(field.min_y() + 4.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    support::click(&mut test, "Passage actions ▾")?;
    key(
        &mut test,
        Key::Character("j".into()),
        Code::KeyJ,
        Modifiers::empty(),
    );
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Add choice")
    );
    Ok(())
}

#[test]
fn reply_destination_is_disclosed_and_escape_returns_to_its_trigger()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    assert!(!has(&test, "Find a destination…"));
    support::click(&mut test, "Change destination…")?;
    test.write_text("station history");
    test.poll_n(std::time::Duration::from_millis(16), 6);
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    test.write_text("station history");
    test.poll_n(std::time::Duration::from_millis(16), 6);
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(!has(&test, "▸ Missing Courier"));
    assert!(has(&test, "▸ Station History"));
    Ok(())
}

#[test]
fn settings_discloses_technical_details_and_contextual_keymap_help()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1000., 900.), |_| {}, 1.).0;
    support::click(&mut test, "Settings")?;
    assert!(has(&test, "Appearance") && has(&test, "Editing") && has(&test, "Behaviour"));
    assert!(!has(&test, "Vim: h j k l"));
    assert!(has(&test, "Done"));
    support::click(&mut test, "Configuration file")?;
    support::click(&mut test, "Keymap: Vim")?;
    assert!(has(&test, "Vim: h j k l"));
    support::click(&mut test, "Done")?;
    assert!(!has(&test, "Done"));
    Ok(())
}
