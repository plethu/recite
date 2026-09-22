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
    test.poll_n(std::time::Duration::from_millis(16), 4);
}

fn confirmation(test: &TestingRunner) -> bool {
    test.find(|_, element| {
        Label::try_downcast(element).filter(|label| label.text.as_ref() == "Close Recite?")
    })
    .is_some()
}

#[test]
fn quit_shortcut_from_prose_opens_a_named_confirmation_and_escape_cancels()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1200., 800.),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    open_script(&mut test)?;
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.starts_with("If you're here")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(field.min_x() + 3.), f64::from(field.min_y() + 3.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    let previous = *platform.focused_accessibility_id.peek();
    assert_ne!(previous.0, 0);
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    key(&mut test, Key::Character("q".into()), Code::KeyQ, modifier);
    assert!(confirmation(&test));
    assert!(
        test.find(|_, element| Rect::try_downcast(element).filter(|rect| rect
            .accessibility
            .builder
            .role()
            == AccessibilityRole::CheckBox
            && rect.accessibility.builder.label() == Some("Don't ask again when closing Recite")))
            .is_some()
    );
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!confirmation(&test));
    assert_eq!(*platform.focused_accessibility_id.peek(), previous);
    Ok(())
}

#[test]
fn escape_never_opens_exit_confirmation_with_or_without_focus()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1200., 800.),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    assert_eq!(platform.focused_accessibility_id.peek().0, 0);
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!confirmation(&test));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!confirmation(&test));
    open_script(&mut test)?;
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.starts_with("If you're here")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(field.min_x() + 3.), f64::from(field.min_y() + 3.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(!confirmation(&test));
    Ok(())
}

#[test]
fn quit_shortcut_reaches_confirmation_from_source_editor() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1200., 800.),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    let source = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == "Source")
                .map(|_| node.layout().area)
        })
        .ok_or("Source control")?;
    test.click_cursor((
        f64::from(source.min_x() + 2.),
        f64::from(source.min_y() + 2.),
    ));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text.contains("Original writer fixture"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("source field")?;
    test.click_cursor((f64::from(field.min_x() + 2.), f64::from(field.min_y() + 2.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_ne!(platform.focused_accessibility_id.peek().0, 0);
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    key(&mut test, Key::Character("q".into()), Code::KeyQ, modifier);
    assert!(confirmation(&test));
    Ok(())
}

fn open_script(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    support::open_beat(test)
}
