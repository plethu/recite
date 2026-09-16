use freya::prelude::*;
use freya_testing::prelude::*;

fn named_area(test: &TestingRunner, name: &str) -> Option<Area> {
    test.find(|node, element| {
        Rect::try_downcast(element)
            .filter(|r| r.accessibility.builder.label() == Some(name))
            .map(|_| node.layout().area)
    })
}

fn caption_area(test: &TestingRunner, caption: &str) -> Option<Area> {
    test.find(|node, element| {
        Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == caption)
            .map(|_| node.layout().area)
    })
}

#[test]
fn connections_expose_endpoints_and_navigate_with_a_pointer_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 6);
    let fit = caption_area(&test, "Fit").ok_or("Fit")?;
    test.move_cursor((f64::from(fit.center().x), f64::from(fit.center().y)));
    test.sync_and_update();
    assert_eq!(test.cursor_icon(), CursorIcon::Pointer);
    assert!(named_area(&test, "Outgoing connections").is_some());
    assert!(named_area(&test, "Incoming connections").is_some());
    assert!(caption_area(&test, "Relay Desk → Missing Courier").is_some());
    let action = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| {
                    r.accessibility
                        .builder
                        .label()
                        .is_some_and(|name| name.starts_with("Show Missing Courier on map."))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("navigation action")?;
    let point = (f64::from(action.center().x), f64::from(action.center().y));
    test.move_cursor(point);
    test.poll_n(std::time::Duration::from_millis(16), 15);
    assert_eq!(test.cursor_icon(), CursorIcon::Pointer);
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join("connection-hover.png"));
    }
    test.click_cursor(point);
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(caption_area(&test, "Connections · Missing Courier").is_some());
    let target = named_area(&test, "Edit Missing Courier").ok_or("target card")?;
    assert!(target.min_x() >= 0. && target.min_y() >= 0.);
    assert!(target.max_x() <= 1440. && target.max_y() <= action.min_y());
    Ok(())
}

#[test]
fn scrolling_over_a_card_pans_and_control_scroll_zooms_at_the_pointer()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 6);
    let before = named_area(&test, "Edit Relay Desk").ok_or("entry card")?;
    let point = (f64::from(before.center().x), f64::from(before.center().y));
    test.scroll(point, (30., -20.));
    test.sync_and_update();
    let panned = named_area(&test, "Edit Relay Desk").ok_or("panned card")?;
    assert!((panned.min_x() - before.min_x() - 30.).abs() < 1.);
    assert!((panned.min_y() - before.min_y() + 20.).abs() < 1.);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Control),
        code: Code::ControlLeft,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    let anchor = (f64::from(panned.center().x), f64::from(panned.center().y));
    test.scroll(anchor, (0., 40.));
    test.sync_and_update();
    let zoomed = named_area(&test, "Edit Relay Desk").ok_or("zoomed card")?;
    assert!(zoomed.width() > panned.width());
    assert!((zoomed.center().x - panned.center().x).abs() < 1.);
    assert!((zoomed.center().y - panned.center().y).abs() < 1.);
    Ok(())
}

#[test]
fn continuous_pan_preserves_card_hit_targets_and_zoom_alignment()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    let before = named_area(&test, "Edit Relay Desk").ok_or("entry card")?;
    let minus = named_area(&test, "Zoom out").ok_or("zoom out")?;
    let plus = named_area(&test, "Zoom in").ok_or("zoom in")?;
    let percentage = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|l| l.text.as_ref() == "100%" && node.layout().area.min_x() < plus.min_x())
                .map(|_| node.layout().area)
        })
        .ok_or("zoom percentage")?;
    assert!((minus.center().y - percentage.center().y).abs() < 1.);
    assert!((plus.center().y - percentage.center().y).abs() < 1.);

    let map = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| {
                    r.accessibility
                        .builder
                        .label()
                        .is_some_and(|name| name.starts_with("Scene map."))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("map viewport")?;
    let start = (f64::from(map.min_x() + 10.), f64::from(map.min_y() + 10.));
    test.press_cursor(start);
    test.sync_and_update();
    for step in 1..=120 {
        test.move_cursor((start.0 + f64::from(step) * 2., start.1 - f64::from(step)));
        test.sync_and_update();
    }
    test.release_cursor((start.0 + 240., start.1 - 120.));
    test.sync_and_update();
    let after = named_area(&test, "Edit Relay Desk").ok_or("panned card")?;
    assert!((after.min_x() - before.min_x() - 240.).abs() < 1.);
    assert!((after.min_y() - before.min_y() + 120.).abs() < 1.);
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join("continuous-pan.png"));
    }
    // A card moved by its parent must still receive clicks at its new coordinates.
    test.click_cursor((f64::from(after.center().x), f64::from(after.max_y() - 10.)));
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(
        test.find(|_, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("If you're here")))
        })
        .is_some()
    );
    Ok(())
}

#[test]
fn dialogue_and_replies_remain_visible_at_seventy_percent() -> Result<(), Box<dyn std::error::Error>>
{
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 6);
    for _ in 0..2 {
        let minus = named_area(&test, "Zoom out").ok_or("zoom out")?;
        test.click_cursor((f64::from(minus.center().x), f64::from(minus.center().y)));
        test.poll_n(std::time::Duration::from_millis(16), 6);
    }
    for caption in [
        "If you're here about the transmitter",
        "Anything I can do to help?",
        "What happened to this place?",
    ] {
        assert!(
            test.find(|_, element| Label::try_downcast(element)
                .filter(|label| label.text.starts_with(caption)))
                .is_some(),
            "{caption}"
        );
    }
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join("hub-seventy-percent.png"));
    }
    Ok(())
}
