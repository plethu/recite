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
