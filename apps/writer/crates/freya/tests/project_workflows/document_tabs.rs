use super::*;

fn key(test: &mut TestingRunner, key: NamedKey, code: Code) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(key),
        code,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 8);
}

fn save_close(test: &mut TestingRunner) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility.builder.role() == AccessibilityRole::Button
                        && r.accessibility.builder.label() == Some("Save and close document")
                })
                .map(|_| node.layout().area)
        })
        .ok_or("save and close button")?;
    test.click_cursor(area.center().to_f64());
    test.poll_n(std::time::Duration::from_millis(16), 15);
    Ok(())
}

#[test]
fn dirty_active_tab_close_can_cancel_and_then_save_before_switching()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "A")?;
    support::click(&mut test, "Source")?;
    let original = std::fs::read_to_string(dir.path().join("a.recite"))?;
    let changed = original.replace("Hello.", "Edited before closing.");
    fill(&mut test, "Hello.", &changed)?;
    support::click(&mut test, "Close a.recite")?;
    assert!(has(&test, "Save and close document"));
    support::click(&mut test, "Cancel")?;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        original
    );
    assert!(has(&test, "Edited before closing."));
    support::click(&mut test, "Close a.recite")?;
    save_close(&mut test)?;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        changed
    );
    assert!(!has(&test, "Save and close document"));
    assert!(has(&test, "Elsewhere"));
    Ok(())
}

#[test]
fn saving_an_inactive_dirty_tab_preserves_the_active_document()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Source")?;
    let original = std::fs::read_to_string(dir.path().join("a.recite"))?;
    let changed = original.replace("Hello.", "Retained inactive draft.");
    fill(&mut test, "Hello.", &changed)?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "Close a.recite")?;
    save_close(&mut test)?;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        changed
    );
    assert!(has(&test, "elsewhere"));
    assert!(!has(&test, "Retained inactive draft."));
    Ok(())
}

#[test]
fn tab_keyboard_navigation_wraps_and_home_end_choose_open_documents()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "a.recite")?;
    key(&mut test, NamedKey::ArrowLeft, Code::ArrowLeft);
    assert!(has(&test, "Elsewhere"));
    key(&mut test, NamedKey::ArrowRight, Code::ArrowRight);
    assert!(has(&test, "Hello."));
    key(&mut test, NamedKey::End, Code::End);
    assert!(has(&test, "Elsewhere"));
    key(&mut test, NamedKey::Home, Code::Home);
    assert!(has(&test, "Hello."));
    Ok(())
}

#[test]
fn a_save_conflict_keeps_the_dirty_tab_and_close_dialog_open()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "A")?;
    support::click(&mut test, "Source")?;
    let original = std::fs::read_to_string(dir.path().join("a.recite"))?;
    fill(
        &mut test,
        "Hello.",
        &original.replace("Hello.", "My draft."),
    )?;
    std::fs::write(
        dir.path().join("a.recite"),
        original.replace("Hello.", "Disk edit."),
    )?;
    support::click(&mut test, "Close a.recite")?;
    save_close(&mut test)?;
    assert!(has(&test, "Save and close document"));
    assert!(has(&test, "changed on disk"));
    assert!(std::fs::read_to_string(dir.path().join("a.recite"))?.contains("Disk edit."));
    support::click(&mut test, "Cancel")?;
    assert!(has(&test, "My draft."));
    Ok(())
}
