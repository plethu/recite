mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: key.clone(),
        code,
        modifiers,
    });
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyUp,
        key,
        code,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(Duration::from_millis(16), 8);
}
fn fill(test: &mut TestingRunner, needle: &str, value: &str) -> Result {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(needle)))
                .map(|_| node.layout().area)
        })
        .ok_or("text input")?;
    test.click_cursor((f64::from(area.min_x() + 3.), f64::from(area.min_y() + 3.)));
    key(
        test,
        Key::Character("a".into()),
        Code::KeyA,
        if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    );
    test.write_text(value);
    test.poll_n(Duration::from_millis(16), 8);
    Ok(())
}
fn alias(test: &mut TestingRunner, alias: &str) -> Result {
    support::click(test, "Commands")?;
    test.write_text(alias);
    test.poll_n(Duration::from_millis(16), 8);
    key(
        test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    Ok(())
}
#[test]
fn write_and_write_all_aliases_persist_edits_through_existing_save_commands() -> Result {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    let a = dir.path().join("a.recite");
    let b = dir.path().join("b.recite");
    std::fs::write(
        &a,
        ":: start default\n> first@11111111111111111111\n  First scene.\n-> END\n",
    )?;
    std::fs::write(
        &b,
        ":: other default\n> second@22222222222222222222\n  Second scene.\n-> END\n",
    )?;
    let mut test =
        TestingRunner::new(recite_writer::editor_app, (1400., 1000.).into(), |_| {}, 1.).0;
    fill(&mut test, "Project folder", &dir.path().to_string_lossy())?;
    support::click(&mut test, "Open project")?;
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keymap: Vim")?;
    support::click(&mut test, "Done")?;
    support::open_beat(&mut test)?;
    fill(&mut test, "First scene.", "First saved edit.")?;
    alias(&mut test, "w")?;
    assert!(std::fs::read_to_string(&a)?.contains("First saved edit."));
    fill(&mut test, "First saved edit.", "First batch edit.")?;
    support::click(&mut test, "B")?;
    fill(&mut test, "Second scene.", "Second batch edit.")?;
    alias(&mut test, "wa")?;
    assert!(std::fs::read_to_string(&a)?.contains("First batch edit."));
    assert!(std::fs::read_to_string(&b)?.contains("Second batch edit."));
    fill(&mut test, "Second batch edit.", "Keep this unsaved draft.")?;
    alias(&mut test, "q")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == "Unsaved changes"))
            .is_some()
    );
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("Keep this unsaved draft."))))
            .is_some()
    );
    assert!(!std::fs::read_to_string(&b)?.contains("Keep this unsaved draft."));
    Ok(())
}
