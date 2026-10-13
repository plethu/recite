mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key,
        code,
        modifiers,
    });
    test.poll_n(std::time::Duration::from_millis(16), 5);
}
fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|label| label.text.as_ref() == text))
        .is_some()
}
fn search_and_activate(
    test: &mut TestingRunner,
    query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text.contains("Find scene or beat…"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("scene filter")?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text(query);
    test.poll_n(std::time::Duration::from_millis(16), 5);
    // The scene header precedes the matching beat in source order.
    for _ in 0..2 {
        key(
            test,
            Key::Named(NamedKey::ArrowDown),
            Code::ArrowDown,
            Modifiers::empty(),
        );
    }
    key(
        test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    Ok(())
}

#[test]
fn renamed_and_switched_scenes_activate_beats_from_their_current_caption_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Rename beat")?;
    key(
        &mut test,
        Key::Character("a".into()),
        Code::KeyA,
        if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    );
    test.write_text("relay_counter");
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has_text(&test, "Relay Counter"));
    search_and_activate(&mut test, "relay counter")?;
    assert!(has_text(&test, "Relay Counter"));
    support::click(&mut test, "Clear Find scene or beat…")?;
    support::click(&mut test, "Last Tram Linear")?;
    search_and_activate(&mut test, "last tram")?;
    assert!(has_text(&test, "Last Tram"));
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|span| span.text.contains("Is this the last tram?"))))
            .is_some()
    );
    support::click(&mut test, "Clear Find scene or beat…")?;
    support::click(&mut test, "Relay Hub")?;
    search_and_activate(&mut test, "station history")?;
    assert!(has_text(&test, "Station History"));
    assert!(!has_text(&test, "Last Tram"));
    Ok(())
}
