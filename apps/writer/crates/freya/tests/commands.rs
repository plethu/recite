mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn command(test: &TestingRunner, title: &str) -> Option<Area> {
    test.find(|node, e| {
        Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some(title))
            .map(|_| node.layout().area)
    })
}

#[test]
fn palette_manual_scroll_stays_put_until_query_or_keyboard_selection_changes() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Commands")?;
    let first = command(&test, "Script").ok_or("first command")?;
    let point = (
        f64::from(first.center().x),
        f64::from(first.center().y + 100.),
    );
    test.scroll(point, (0., -220.));
    test.poll_n(Duration::from_millis(16), 30);
    let rows = |test: &TestingRunner| {
        test.find_many(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility.builder.label().is_some_and(|name| {
                        [
                            "Script",
                            "Map",
                            "Source",
                            "Add beat",
                            "Add line",
                            "Add reply",
                            "Settings",
                        ]
                        .contains(&name)
                    })
                })
                .map(|r| {
                    (
                        r.accessibility.builder.label().unwrap().to_owned(),
                        node.layout().area,
                    )
                })
        })
    };
    let scrolled = rows(&test);
    assert!(command(&test, "Script").is_none_or(|area| area.min_y() < first.min_y() - 50.));
    test.poll_n(Duration::from_millis(16), 30);
    assert_eq!(
        rows(&test),
        scrolled,
        "idle layout must not reset manual scrolling"
    );
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join("palette-scrolled.png"));
    }
    test.write_text("source");
    test.poll_n(Duration::from_millis(16), 8);
    let result = command(&test, "Source").ok_or("filtered command")?;
    assert!((result.min_y() - first.min_y()).abs() < 2.);
    test.press_key(Key::Named(NamedKey::ArrowDown));
    test.press_key(Key::Named(NamedKey::Enter));
    test.poll_n(Duration::from_millis(16), 6);
    assert!(command(&test, "Source editor").is_some());
    Ok(())
}

#[test]
fn settings_backdrop_dismisses_but_clicking_the_surface_does_not() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1000., 800.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Settings")?;
    let dialog = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.role() == AccessibilityRole::Dialog)
                .map(|_| node.layout().area)
        })
        .ok_or("settings dialog")?;
    test.click_cursor((
        f64::from(dialog.min_x() + 15.),
        f64::from(dialog.min_y() + 15.),
    ));
    test.poll_n(Duration::from_millis(16), 4);
    assert!(command(&test, "Done").is_some());
    test.click_cursor((5., 5.));
    test.poll_n(Duration::from_millis(16), 4);
    assert!(command(&test, "Done").is_none());
    assert!(command(&test, "Beat editor").is_some());
    Ok(())
}

#[test]
fn small_scroll_steps_stay_continuous_across_virtual_row_boundaries() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Commands")?;
    let first = command(&test, "Script").ok_or("first command")?;
    let point = (
        f64::from(first.center().x),
        f64::from(first.center().y + 100.),
    );
    let mut previous = command(&test, "Source").ok_or("source command")?.min_y();
    for _ in 0..12 {
        test.scroll(point, (0., -6.));
        test.poll_n(Duration::from_millis(16), 20);
        let next = command(&test, "Source")
            .ok_or("source stays in viewport")?
            .min_y();
        assert!(
            (previous - next - 6.).abs() < 1.,
            "row jumped: {previous} -> {next}"
        );
        previous = next;
    }
    Ok(())
}

fn send_key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
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
    test.poll_n(Duration::from_millis(16), 12);
}

#[test]
fn keyboard_settings_record_binding_and_update_dispatch() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 1000.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keyboard shortcuts")?;
    let binding = if cfg!(target_os = "macos") {
        "Cmd+Shift+P"
    } else {
        "Ctrl+Shift+P"
    };
    support::click(&mut test, &format!("Rebind Commands: {binding}"))?;
    let primary = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    send_key(
        &mut test,
        Key::Named(NamedKey::F8),
        Code::F8,
        Modifiers::empty(),
    );
    support::click(&mut test, "Save binding")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == "Shortcut saved."))
            .is_some()
    );
    support::click(&mut test, "Done")?;
    send_key(
        &mut test,
        Key::Character("p".into()),
        Code::KeyP,
        primary | Modifiers::SHIFT,
    );
    assert!(command(&test, "Close commands").is_none());
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Source editor")?;
    send_key(
        &mut test,
        Key::Named(NamedKey::F8),
        Code::F8,
        Modifiers::empty(),
    );
    assert!(command(&test, "Close commands").is_some());
    Ok(())
}

#[test]
fn modifier_hints_do_not_move_controls_and_disappear_on_release() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    test.poll_n(Duration::from_millis(16), 10);
    let prose = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.starts_with("If you're here")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(prose.min_x() + 3.), f64::from(prose.min_y() + 3.)));
    let before = command(&test, "Commands").ok_or("commands action")?;
    let (modifier, named, code, chip) = if cfg!(target_os = "macos") {
        (Modifiers::META, NamedKey::Meta, Code::MetaLeft, "Cmd+1")
    } else {
        (
            Modifiers::CONTROL,
            NamedKey::Control,
            Code::ControlLeft,
            "Ctrl+1",
        )
    };
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(named),
        code,
        modifiers: modifier,
    });
    test.poll_n(Duration::from_millis(16), 10);
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == chip))
            .is_some()
    );
    assert_eq!(command(&test, "Commands"), Some(before));
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        test.render_to_file(std::path::Path::new(&directory).join("shortcut-hints.png"));
    }
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyUp,
        key: Key::Named(named),
        code,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(Duration::from_millis(16), 10);
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == chip))
            .is_none()
    );
    Ok(())
}

#[test]
fn settings_keyboard_focus_reveals_lower_controls_and_monochrome_updates_live() -> Result {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1000., 700.).into(),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Monochrome colours")?;
    // The window's active theme changes without needing to reopen a document.
    let colors =
        test.find_many(|_, e| Rect::try_downcast(e).and_then(|r| r.style.background.as_color()));
    assert!(
        colors
            .iter()
            .filter(|c| c.a() == 255)
            .all(|c| c.r() == c.g() && c.g() == c.b())
    );
    for _ in 0..24 {
        send_key(
            &mut test,
            Key::Named(NamedKey::Tab),
            Code::Tab,
            Modifiers::empty(),
        );
        let target = test.find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Always show shortcut hints"))
                .map(|_| node.layout().area)
        });
        if platform.focused_accessibility_node.peek().label() == Some("Always show shortcut hints")
        {
            let area = target.ok_or("focused hint setting")?;
            assert!(
                area.min_y() > 120. && area.max_y() < 570.,
                "focused control outside dialog: {area:?}"
            );
            send_key(
                &mut test,
                Key::Character(" ".into()),
                Code::Space,
                Modifiers::empty(),
            );
            support::click(&mut test, "Done")?;
            let chip = if cfg!(target_os = "macos") {
                "Cmd+1"
            } else {
                "Ctrl+1"
            };
            assert!(
                test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == chip))
                    .is_some()
            );
            if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
                test.render_to_file(std::path::Path::new(&directory).join("monochrome.png"));
            }
            return Ok(());
        }
    }
    Err("lower preference was never revealed".into())
}
