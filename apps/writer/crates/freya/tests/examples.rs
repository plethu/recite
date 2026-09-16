mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn click(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|rect| rect.accessibility.builder.label() == Some(caption))
                .map(|_| node.layout().area)
                .or_else(|| {
                    Label::try_downcast(element)
                        .filter(|label| label.text.as_ref() == caption)
                        .map(|_| node.layout().area)
                })
        })
        .ok_or("visible control")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}

#[test]
fn default_examples_switch_without_losing_field_drafts_or_applied_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 800.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("Replies")))
            .is_some()
    );
    test.poll_n(std::time::Duration::from_millis(16), 6);
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join("hub-script.png"));
    }
    support::dark_theme(&mut test)?;
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        test.render_to_file(std::path::Path::new(&directory).join("hub-script-dark.png"));
    }
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Theme: Light")?;
    support::click(&mut test, "Close settings")?;
    click(&mut test, "Floodgate Waterfall")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("I'll ring"))))
            .is_some()
    );
    click(&mut test, "Last Tram Linear")?;
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text.contains("Is this the last tram?"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("prose input")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("Wait for me!");
    click(&mut test, "Relay Hub")?;
    click(&mut test, "Last Tram Linear")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "Wait for me!")))
            .is_some()
    );
    click(&mut test, "Floodgate Waterfall")?;
    click(&mut test, "Last Tram Linear")?;
    click(&mut test, "Undo")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "Is this the last tram?")))
            .is_some()
    );
    Ok(())
}
