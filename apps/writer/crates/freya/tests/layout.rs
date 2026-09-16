mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn area(test: &TestingRunner, name: &str) -> Option<Area> {
    test.find(|node, e| {
        Rect::try_downcast(e)
            .filter(|r| {
                r.accessibility
                    .builder
                    .label()
                    .is_some_and(|label| label.starts_with(name))
            })
            .map(|_| node.layout().area)
    })
}
fn drag(test: &mut TestingRunner, name: &str, dx: f64) -> Result<(), Box<dyn std::error::Error>> {
    let divider = area(test, name).ok_or("divider")?;
    let start = (f64::from(divider.center().x), f64::from(divider.center().y));
    test.press_cursor(start);
    test.sync_and_update();
    test.move_cursor((start.0 + dx, start.1));
    test.sync_and_update();
    test.release_cursor((start.0 + dx, start.1));
    test.sync_and_update();
    Ok(())
}

#[test]
fn resizing_keeps_text_layout_stable_until_release_and_escape_cancels()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    let original = area(&test, "Beat editor").ok_or("editor")?;
    let divider = area(&test, "Resize script pane").ok_or("divider")?;
    let start = (f64::from(divider.center().x), f64::from(divider.center().y));
    test.press_cursor(start);
    for step in 1..=10 {
        test.move_cursor((start.0 - f64::from(step * 10), start.1));
        test.sync_and_update();
        assert_eq!(area(&test, "Beat editor"), Some(original));
    }
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("resize-guide.png"));
    }
    test.release_cursor((start.0 - 100., start.1));
    test.sync_and_update();
    assert_eq!(
        area(&test, "Beat editor").ok_or("resized editor")?.width(),
        original.width() + 100.
    );
    let resized = area(&test, "Beat editor");
    let divider = area(&test, "Resize script pane").ok_or("divider")?;
    let start = (f64::from(divider.center().x), f64::from(divider.center().y));
    test.press_cursor(start);
    test.move_cursor((start.0 + 100., start.1));
    test.sync_and_update();
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Escape),
        code: Code::Escape,
        modifiers: Modifiers::empty(),
    });
    test.sync_and_update();
    test.release_cursor((start.0 + 100., start.1));
    test.sync_and_update();
    assert_eq!(area(&test, "Beat editor"), resized);
    Ok(())
}

#[test]
fn pane_resize_is_bounded_and_script_side_changes_without_losing_the_beat()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    drag(&mut test, "Resize scene drawer", 900.)?;
    assert_eq!(
        area(&test, "Scene navigation").ok_or("drawer")?.width(),
        360.
    );
    drag(&mut test, "Resize scene drawer", -900.)?;
    assert_eq!(
        area(&test, "Scene navigation").ok_or("drawer")?.width(),
        180.
    );
    support::open_beat(&mut test)?;
    drag(&mut test, "Resize script pane", 900.)?;
    assert_eq!(area(&test, "Beat editor").ok_or("editor")?.width(), 320.);
    drag(&mut test, "Resize script pane", -900.)?;
    assert_eq!(area(&test, "Beat editor").ok_or("editor")?.width(), 800.);
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Script pane: Left")?;
    support::click(&mut test, "Close settings")?;
    let script = area(&test, "Beat editor").ok_or("editor")?;
    let map = area(&test, "Scene map.").ok_or("map")?;
    assert!(script.max_x() < map.min_x());
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.starts_with("If you're here"))))
            .is_some()
    );
    drag(&mut test, "Resize script pane", -900.)?;
    assert_eq!(area(&test, "Beat editor").ok_or("editor")?.width(), 320.);
    support::click(&mut test, "Resize script pane")?;
    for (code, key, expected) in [
        (Code::ArrowRight, NamedKey::ArrowRight, 336.),
        (Code::End, NamedKey::End, 800.),
        (Code::Home, NamedKey::Home, 320.),
    ] {
        test.send_event(PlatformEvent::Keyboard {
            name: KeyboardEventName::KeyDown,
            code,
            key: Key::Named(key),
            modifiers: Modifiers::empty(),
        });
        test.sync_and_update();
        assert_eq!(
            area(&test, "Beat editor").ok_or("editor")?.width(),
            expected
        );
    }
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("script-left.png"));
    }
    Ok(())
}

#[test]
fn arrange_keeps_zoom_and_drawer_reveal_keeps_text_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    support::click(&mut test, "Zoom out")?;
    let width = area(&test, "Edit Relay Desk").ok_or("card")?.width();
    support::click(&mut test, "Arrange automatically")?;
    assert_eq!(area(&test, "Edit Relay Desk").ok_or("card")?.width(), width);
    support::click(&mut test, "Hide scenes")?;
    let show = area(&test, "Show scenes").ok_or("toggle")?;
    test.click_cursor((f64::from(show.center().x), f64::from(show.center().y)));
    let mut sizes = Vec::new();
    for _ in 0..12 {
        test.poll_n(std::time::Duration::from_millis(16), 1);
        sizes.push(area(&test, "Relay Hub").ok_or("scene row")?.size);
    }
    assert!(sizes.iter().all(|size| size == &sizes[0]));
    Ok(())
}

#[test]
fn shared_buttons_use_pointer_cursors_and_modals_fit_a_small_window()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(900., 650.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    support::open_beat(&mut test)?;
    for caption in ["Rename beat", "Close beat editor"] {
        let button = test
            .find(|node, e| {
                Rect::try_downcast(e)
                    .filter(|r| r.accessibility.builder.label() == Some(caption))
                    .map(|_| node.layout().area)
            })
            .ok_or("button")?;
        test.move_cursor((f64::from(button.center().x), f64::from(button.center().y)));
        test.sync_and_update();
        assert_eq!(test.cursor_icon(), CursorIcon::Pointer);
    }
    support::click(&mut test, "Settings")?;
    let settings = area(&test, "Settings").ok_or("settings")?;
    assert!(settings.min_y() >= 0. && settings.max_y() <= 650.);
    let close = area(&test, "Close settings").ok_or("close settings")?;
    assert!(close.max_y() <= 650.);
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("settings-small.png"));
    }
    support::click(&mut test, "Close settings")?;
    recite_writer::request_close();
    test.poll_n(std::time::Duration::from_millis(16), 12);
    let keep = area(&test, "Keep editing").ok_or("keep editing")?;
    let close = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Close Recite"))
                .map(|_| node.layout().area)
        })
        .ok_or("close")?;
    assert!((keep.center().y - close.center().y).abs() < 1.);
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        test.render_to_file(std::path::Path::new(&dir).join("close-small.png"));
    }
    Ok(())
}
