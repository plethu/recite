//! Large diagnostic lists stay bounded and retain source navigation and keyboard access.
mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;

fn key(test: &mut TestingRunner, key: NamedKey, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(key),
        code: Code::Unidentified,
        modifiers,
    });
    test.poll_n(Duration::from_millis(16), 8);
}
fn mounted(test: &TestingRunner) -> usize {
    test.find_many(|_, element| {
        Rect::try_downcast(element)
            .filter(|rect| {
                rect.accessibility
                    .builder
                    .label()
                    .is_some_and(|label| label.starts_with("Open diagnostic "))
            })
            .map(|_| ())
    })
    .len()
}
fn caption(diagnostic: &recite_core::Diagnostic) -> String {
    format!(
        "Open diagnostic {} at {}:{}:{}",
        diagnostic.code,
        diagnostic.span.file,
        diagnostic.span.start.line(),
        diagnostic.span.start.column()
    )
}
#[test]
fn thousands_of_errors_mount_only_visible_rows_and_last_diagnostic_remains_keyboard_reachable()
-> Result<(), Box<dyn std::error::Error>> {
    let source = (0..1_000)
        .map(|n| {
            let target = if n == 999 {
                format!("missing_{n}_{}", "long".repeat(100))
            } else {
                format!("missing_{n}")
            };
            format!(
                ":: block_{n}{}\n> line@{n:020x}\n  Prose.\n-> {target}\n",
                if n == 0 { " default" } else { "" }
            )
        })
        .collect::<String>();
    let document = recite_writer_model::Document::new(&source)?;
    let all = document.diagnostics();
    assert_eq!(all.len(), 1_000);
    let last = all.last().ok_or("last diagnostic")?;
    let last_caption = caption(last);
    let line = source
        .lines()
        .nth(last.span.start.line() as usize - 1)
        .ok_or("source line")?;
    let column = last.span.start.column() as usize - 1;
    let expected = format!(
        "{}X{}",
        line.chars().take(column).collect::<String>(),
        line.chars().skip(column).collect::<String>()
    );
    let (mut test, mut platform) = TestingRunner::new(
        recite_writer::regression_app,
        (900., 650.).into(),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialSource(source));
            runner.provide_root_context(|| Platform::get().current_window())
        },
        1.,
    );
    assert!(mounted(&test) <= 20);
    support::click(&mut test, "Source")?;
    assert!((1..=20).contains(&mounted(&test)));
    for _ in 0..50 {
        if platform.focused_accessibility_node.peek().label() == Some("Diagnostics") {
            break;
        }
        key(&mut test, NamedKey::Tab, Modifiers::empty());
    }
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Diagnostics")
    );
    // Enter an actual virtual row before jumping, as native Tab navigation does.
    key(&mut test, NamedKey::Tab, Modifiers::empty());
    assert!(
        platform
            .focused_accessibility_node
            .peek()
            .label()
            .is_some_and(|label| label.starts_with("Open diagnostic "))
    );
    platform.navigation_mode.set(NavigationMode::Keyboard);
    key(&mut test, NamedKey::PageDown, Modifiers::empty());
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some(caption(&all[3]).as_str())
    );
    key(&mut test, NamedKey::PageUp, Modifiers::empty());
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some(caption(&all[0]).as_str())
    );
    key(&mut test, NamedKey::End, Modifiers::empty());
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some(last_caption.as_str()),
        "mounted: {:?}",
        test.find_many(|_, e| Rect::try_downcast(e).and_then(|r| r
            .accessibility
            .builder
            .label()
            .filter(|l| l.starts_with("Open diagnostic "))
            .map(str::to_owned)))
    );
    assert!((1..=20).contains(&mounted(&test)));
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == last.message && label.max_lines.is_none()))
            .is_some()
    );
    if let Ok(directory) = std::env::var("RECITE_DIAGNOSTICS_SCREENSHOTS") {
        test.render_to_file(std::path::Path::new(&directory).join("diagnostics-narrow-light.png"));
        key(&mut test, NamedKey::Escape, Modifiers::empty());
        support::dark_theme(&mut test)?;
        for _ in 0..50 {
            if platform.focused_accessibility_node.peek().label() == Some("Diagnostics") {
                break;
            }
            key(&mut test, NamedKey::Tab, Modifiers::empty());
        }
        platform.navigation_mode.set(NavigationMode::Keyboard);
        key(&mut test, NamedKey::End, Modifiers::empty());
        test.render_to_file(std::path::Path::new(&directory).join("diagnostics-narrow-dark.png"));
        key(&mut test, NamedKey::Escape, Modifiers::empty());
        support::click(&mut test, &last_caption)?;
    } else {
        key(&mut test, NamedKey::Escape, Modifiers::empty());
        assert!(
            test.find(
                |_, element| Label::try_downcast(element).filter(|label| label.text.as_ref()
                    == last.message
                    && label.max_lines.is_none())
            )
            .is_none()
        );
        key(&mut test, NamedKey::Enter, Modifiers::empty());
    }
    test.write_text("X");
    test.poll_n(Duration::from_millis(16), 4);
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .map(|span| span.text.as_ref())
                .collect::<String>()
                .contains(&expected))
        )
        .is_some()
    );
    assert!(
        test.find(|_, element| Rect::try_downcast(element).filter(|rect| rect
            .accessibility
            .builder
            .description()
            == Some(last.message.as_str())))
            .is_some()
    );
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.write_text(":: fixed default\n-> END\n");
    test.poll_n(Duration::from_millis(16), 4);
    key(&mut test, NamedKey::Enter, Modifiers::CONTROL);
    assert_eq!(mounted(&test), 0);
    Ok(())
}
