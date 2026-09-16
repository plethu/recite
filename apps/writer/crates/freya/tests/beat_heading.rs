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
fn replace(test: &mut TestingRunner, text: &str) {
    key(
        test,
        Key::Character("a".into()),
        Code::KeyA,
        Modifiers::CONTROL,
    );
    test.write_text(text);
}
fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == text))
        .is_some()
}

#[test]
fn inline_rename_can_be_cancelled_validated_and_undone() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 900.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Rename beat")?;
    replace(&mut test, "abandoned_title");
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(has_text(&test, "Relay Desk"));
    support::click(&mut test, "Rename beat")?;
    replace(&mut test, "station_history");
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    // A duplicate identifier keeps the edit open instead of dismissing the pane.
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some("Apply beat name")))
            .is_some()
    );
    replace(&mut test, "relay_counter");
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has_text(&test, "Relay Counter"));
    assert!(
        test.find(
            |_, e| Rect::try_downcast(e).filter(|r| r.accessibility.builder.label()
                == Some("Start · Relay Counter")
                && r.accessibility.builder.toggled() == Some(accesskit::Toggled::True))
        )
        .is_some()
    );
    support::click(&mut test, "Undo")?;
    assert!(has_text(&test, "Relay Desk"));
    support::click(&mut test, "Redo")?;
    assert!(has_text(&test, "Relay Counter"));
    assert!(
        test.find(
            |_, e| Rect::try_downcast(e).filter(|r| r.accessibility.builder.label()
                == Some("Start · Relay Counter")
                && r.accessibility.builder.toggled() == Some(accesskit::Toggled::True))
        )
        .is_some()
    );
    support::click(&mut test, "Close beat editor")?;
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some("Rename beat")))
            .is_none()
    );
    Ok(())
}
