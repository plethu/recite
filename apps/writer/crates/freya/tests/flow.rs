mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn settle(test: &mut TestingRunner) {
    test.poll_n(std::time::Duration::from_millis(16), 6);
}

fn click(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    settle(test);
    let area = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text.as_ref() == caption)
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("control {caption}"))?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    settle(test);
    Ok(())
}

fn prose(test: &TestingRunner, needle: &str) -> Option<freya::prelude::Area> {
    test.find(|node, element| {
        Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text.contains(needle)))
            .map(|_| node.layout().area)
    })
}

fn capture(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
    Ok(())
}

#[test]
fn map_first_and_independent_branch_previews() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    settle(&mut test);
    assert!(prose(&test, "If you're here").is_none());
    capture(&mut test, "hub-map.png")?;
    support::open_beat(&mut test)?;
    click(&mut test, "▸ Missing Courier")?;
    click(&mut test, "▸ Station History")?;
    for caption in ["Edit Missing Courier", "Edit Station History"] {
        assert!(
            test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == caption))
                .is_some()
        );
    }
    capture(&mut test, "hub-branches.png")?;
    click(&mut test, "▾ Missing Courier")?;
    assert!(
        test.find(
            |_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Edit Station History")
        )
        .is_some()
    );
    click(&mut test, "Map")?;
    click(&mut test, "Last Tram Linear")?;
    capture(&mut test, "linear-map.png")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Last Tram · end"))
            .is_some()
    );
    click(&mut test, "Add beat")?;
    assert!(prose(&test, "New dialogue.").is_some());
    capture(&mut test, "added-beat.png")?;
    Ok(())
}

#[test]
fn prose_focus_keeps_geometry_and_commits_one_undoable_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 800.), |_| {}, 1.).0;
    click(&mut test, "Last Tram Linear")?;
    support::open_beat(&mut test)?;
    let before = prose(&test, "Only if we get on it.").ok_or("second field")?;
    let following = prose(&test, "Hold the door, then.").ok_or("third field")?;
    test.click_cursor((
        f64::from(before.min_x() + 2.),
        f64::from(before.min_y() + 2.),
    ));
    settle(&mut test);
    assert_eq!(prose(&test, "Only if we get on it."), Some(before));
    assert_eq!(prose(&test, "Hold the door, then."), Some(following));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.sync_and_update();
    test.write_text("There is still time.");
    let next = prose(&test, "Hold the door, then.").ok_or("third field")?;
    test.click_cursor((f64::from(next.min_x() + 2.), f64::from(next.min_y() + 2.)));
    settle(&mut test);
    assert!(prose(&test, "There is still time.").is_some());
    click(&mut test, "Source")?;
    assert!(prose(&test, "There is still time.").is_some());
    support::open_beat(&mut test)?;
    // Icon buttons expose their names through accessibility, not visible text.
    let undo = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some("Undo"))
                .map(|_| node.layout().area)
        })
        .ok_or("Undo")?;
    test.click_cursor((f64::from(undo.min_x() + 2.), f64::from(undo.min_y() + 2.)));
    settle(&mut test);
    assert!(prose(&test, "Only if we get on it.").is_some());
    capture(&mut test, "linear-editing.png")?;
    Ok(())
}

#[test]
fn rejected_prose_stays_visible_and_can_be_discarded() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 800.), |_| {}, 1.).0;
    click(&mut test, "Last Tram Linear")?;
    support::open_beat(&mut test)?;
    let area = prose(&test, "Is this the last tram?").ok_or("first field")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    settle(&mut test);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.sync_and_update();
    test.write_text("Text\n-> END");
    click(&mut test, "Source")?;
    assert!(prose(&test, "-> END").is_some());
    assert!(
        test.find(
            |_, element| Label::try_downcast(element).filter(|l| l.text.contains("not applied"))
        )
        .is_some()
    );
    support::click(&mut test, "Dismiss message")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Discard draft"))
            .is_some()
    );
    click(&mut test, "Source")?;
    assert!(prose(&test, "-> END").is_some());
    click(&mut test, "Discard draft")?;
    assert!(prose(&test, "Is this the last tram?").is_some());
    Ok(())
}

#[test]
fn arranging_a_card_tracks_pointer_and_keyboard_without_changing_source()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    settle(&mut test);
    let handle = |test: &TestingRunner| {
        test.find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some("Move Relay Desk"))
                .map(|_| node.layout().area)
        })
    };
    let before = handle(&test).ok_or("move handle")?;
    let point = (
        f64::from(before.min_x() + 4.),
        f64::from(before.min_y() + 4.),
    );
    test.move_cursor(point);
    settle(&mut test);
    capture(&mut test, "grip-hover.png")?;
    assert_eq!(test.cursor_icon(), CursorIcon::Grab);
    test.press_cursor(point);
    test.sync_and_update();
    test.move_cursor((point.0 + 40., point.1 + 24.));
    settle(&mut test);
    test.release_cursor((point.0 + 40., point.1 + 24.));
    settle(&mut test);
    let moved = handle(&test).ok_or("moved handle")?;
    capture(&mut test, "dragged.png")?;
    assert!(
        (moved.min_x() - before.min_x() - 40.).abs() < 1.,
        "before={before:?}, moved={moved:?}"
    );
    assert!((moved.min_y() - before.min_y() - 24.).abs() < 1.);
    test.click_cursor((f64::from(moved.min_x() + 4.), f64::from(moved.min_y() + 4.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::ArrowRight),
        code: Code::ArrowRight,
        modifiers: Modifiers::empty(),
    });
    settle(&mut test);
    let nudged = handle(&test).ok_or("nudged handle")?;
    assert!(nudged.min_x() > moved.min_x());
    click(&mut test, "Arrange automatically")?;
    assert_eq!(handle(&test), Some(before));
    click(&mut test, "Source")?;
    assert!(prose(&test, "31000000000000000001").is_some());
    Ok(())
}

#[test]
fn map_pan_zoom_and_persistent_nudge_use_the_same_visible_coordinates()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let state_path = directory.path().join("writer-layouts.json");
    let path = state_path.clone();
    let mut test = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1440., 1000.),
        move |runner| {
            let _storage = runner
                .provide_root_context(|| Ok::<_, String>(recite_config::UserStateFile::new(path)));
        },
        1.,
    )
    .0;
    settle(&mut test);
    let grip = |test: &TestingRunner| {
        test.find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some("Move Relay Desk"))
                .map(|_| node.layout().area)
        })
    };
    let before = grip(&test).ok_or("grip")?;
    test.move_cursor((
        f64::from(before.min_x() + 4.),
        f64::from(before.min_y() + 4.),
    ));
    settle(&mut test);
    test.click_cursor((
        f64::from(before.min_x() + 4.),
        f64::from(before.min_y() + 4.),
    ));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::ArrowRight),
        code: Code::ArrowRight,
        modifiers: Modifiers::empty(),
    });
    settle(&mut test);
    let moved = grip(&test).ok_or("moved grip")?;
    assert!(moved.min_x() > before.min_x());
    assert!(std::fs::read_to_string(&state_path)?.contains("passage:31000000000000000001"));
    // Pan an empty patch; releasing over the graph must not activate a beat.
    let viewport = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility
                        .builder
                        .label()
                        .is_some_and(|label| label.starts_with("Scene map."))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("viewport")?;
    let start = (
        f64::from(viewport.min_x() + 10.),
        f64::from(viewport.min_y() + 10.),
    );
    test.press_cursor(start);
    test.sync_and_update();
    test.move_cursor((start.0 + 32., start.1 + 16.));
    settle(&mut test);
    test.release_cursor((start.0 + 32., start.1 + 16.));
    settle(&mut test);
    let panned = grip(&test).ok_or("panned grip")?;
    assert!((panned.min_x() - moved.min_x() - 32.).abs() < 1.);
    assert!(prose(&test, "If you're here").is_none());
    click(&mut test, "100%")?;
    click(&mut test, "Fit")?;
    let fitted = grip(&test).ok_or("fitted grip")?;
    assert_ne!(fitted, panned);
    drop(test);
    let mut reopened = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1440., 1000.),
        move |runner| {
            let _storage = runner.provide_root_context(|| {
                Ok::<_, String>(recite_config::UserStateFile::new(state_path))
            });
        },
        1.,
    )
    .0;
    settle(&mut reopened);
    let restored = grip(&reopened).ok_or("restored grip")?;
    assert!((restored.min_x() - moved.min_x()).abs() < 1.);
    Ok(())
}

#[test]
fn map_overview_omits_miniature_dialogue_and_restores_it_at_working_zoom()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    settle(&mut test);
    click(&mut test, "Floodgate Waterfall")?;
    let dialogue = |test: &TestingRunner| {
        test.find(|_, e| {
            Label::try_downcast(e).filter(|l| l.text.starts_with("The lower gate is stuck open"))
        })
    };
    assert!(dialogue(&test).is_some());
    capture(&mut test, "waterfall-working.png")?;
    click(&mut test, "Fit")?;
    assert!(dialogue(&test).is_none());
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Alarm"))
            .is_some()
    );
    capture(&mut test, "waterfall-overview.png")?;
    click(&mut test, "100%")?;
    assert!(dialogue(&test).is_some());
    Ok(())
}
