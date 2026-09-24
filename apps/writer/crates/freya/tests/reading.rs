mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|label| label.text.contains(text)))
        .is_some()
}

fn has_prose(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Paragraph::try_downcast(e).filter(|p| p.spans.iter().any(|s| s.text.contains(text)))
    })
    .is_some()
}

#[test]
fn pinned_context_survives_navigation_and_history() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1400., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Pin reference")?;
    assert!(has_text(&test, "Pinned snapshot"));
    assert!(has_text(&test, "Read only"));
    support::click(&mut test, "Toggle pinned reference")?;
    assert!(!has_text(&test, "Read only"));
    support::click(&mut test, "Toggle pinned reference")?;
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("pinned-reference.png"));
    }
    support::click(&mut test, "Missing Courier")?;
    support::click(&mut test, "Open script")?;
    assert!(has_prose(&test, "Our courier is two days late"));
    support::click(&mut test, "Back")?;
    assert!(has_prose(&test, "If you're here about"));
    assert!(!has_prose(&test, "Our courier is two days late"));
    support::click(&mut test, "Forward")?;
    assert!(has_prose(&test, "Our courier is two days late"));
    assert!(has_text(&test, "Pinned snapshot"));
    support::click(&mut test, "Unpin reference")?;
    assert!(!has_text(&test, "Pinned snapshot"));
    Ok(())
}

#[test]
fn long_beat_pages_commit_prose_and_keep_undo() -> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::workload::source(96, 0, 96);
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1400., 1000.),
        |runner| runner.provide_root_context(move || recite_writer::InitialSource(source)),
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    assert!(has_text(&test, "Passages 1–32"));
    let paragraph = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Beacon0 remains")))
                .map(|_| node.layout().area)
        })
        .ok_or("first passage")?;
    test.click_cursor((
        f64::from(paragraph.min_x() + 2.),
        f64::from(paragraph.min_y() + 2.),
    ));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("A question preserved across pages.");
    support::click(&mut test, "Next passages")?;
    assert!(has_text(&test, "Passages 33–64"));
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Beacon0 remains"))))
            .is_none()
    );
    support::click(&mut test, "Previous passages")?;
    let has_draft = |test: &TestingRunner| {
        test.find(|_, e| {
            Paragraph::try_downcast(e).filter(|p| {
                p.spans
                    .iter()
                    .any(|s| s.text == "A question preserved across pages.")
            })
        })
        .is_some()
    };
    assert!(has_draft(&test));
    support::click(&mut test, "Undo")?;
    assert!(!has_draft(&test));
    support::click(&mut test, "Redo")?;
    assert!(has_draft(&test));
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text == "A question preserved across pages.")
                })
                .map(|_| node.layout().area)
        })
        .ok_or("edited passage")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text(":: syntax_is_not_prose");
    support::click(&mut test, "Next passages")?;
    assert!(has_text(&test, "Passages 1–32"));
    assert!(has_prose(&test, ":: syntax_is_not_prose"));
    Ok(())
}

#[test]
fn long_paragraph_keeps_its_geometry_when_appearance_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let sentence = "The courier waits beside the café. Beacon0 remains unanswered.";
    let source = recite_writer_model::workload::source(24, 0, 1).replacen(
        sentence,
        &format!("{sentence} ").repeat(12),
        1,
    );
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1400., 1000.),
        |runner| runner.provide_root_context(move || recite_writer::InitialSource(source)),
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    let mut sizes = Vec::new();
    for appearance in ["light", "dark"] {
        if appearance == "dark" {
            support::dark_theme(&mut test)?;
        }
        let area = test
            .find(|node, element| {
                Paragraph::try_downcast(element)
                    .filter(|p| {
                        p.spans
                            .iter()
                            .any(|span| span.text.contains("Beacon0 remains"))
                    })
                    .map(|_| node.layout().area)
            })
            .ok_or("long paragraph")?;
        sizes.push(area.size);
        if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
            std::fs::create_dir_all(&directory)?;
            test.render_to_file(
                std::path::Path::new(&directory).join(format!("long-paragraph-{appearance}.png")),
            );
        }
    }
    assert_eq!(sizes[0], sizes[1]);
    Ok(())
}
