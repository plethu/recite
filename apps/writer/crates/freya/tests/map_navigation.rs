use freya::prelude::*;
use freya_testing::prelude::*;

fn named_area(test: &TestingRunner, name: &str) -> Option<Area> {
    test.find(|node, element| {
        Rect::try_downcast(element)
            .filter(|r| r.accessibility.builder.label() == Some(name))
            .map(|_| node.layout().area)
    })
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

    test.press_cursor((260., 200.));
    test.sync_and_update();
    for step in 1..=120 {
        test.move_cursor((260. + f64::from(step) * 2., 200. - f64::from(step)));
        test.sync_and_update();
    }
    test.release_cursor((500., 80.));
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
