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
    support::wait_editor_work(test)?;
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

#[test]
fn source_deep_links_preserve_new_scene_preferences_and_closed_tab_source()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let root = dir.path().to_owned();
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        (1400., 1000.).into(),
        move |runner| {
            runner.provide_root_context(move || recite_writer::InitialProject(Some(root)));
            runner.provide_root_context(|| {
                recite_writer::InitialRoute(
                    "recite://writer/write?scene=a.recite&view=source".into(),
                )
            });
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    let source_visible = |test: &TestingRunner| {
        test.find(|_, element| {
            Rect::try_downcast(element)
                .filter(|rect| rect.accessibility.builder.label() == Some("Source editor"))
        })
        .is_some()
    };
    assert!(source_visible(&test));
    support::click(&mut test, "B")?;
    assert!(
        !source_visible(&test),
        "new scenes use the Script preference, not the old deep link"
    );
    support::click(&mut test, "Close a.recite")?;
    support::click(&mut test, "A")?;
    assert!(
        source_visible(&test),
        "a closed tab restores its remembered Source presentation"
    );
    assert!(
        test.find(|_, element| {
            Paragraph::try_downcast(element).filter(|paragraph| {
                paragraph
                    .spans
                    .iter()
                    .map(|span| span.text.as_ref())
                    .collect::<String>()
                    .contains("> line@11111111111111111111")
            })
        })
        .is_some(),
        "Source must contain the full document rather than the first prose field"
    );
    Ok(())
}

#[test]
fn cancelling_a_submitted_tab_close_keeps_the_tab_while_its_save_finishes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "A")?;
    support::click(&mut test, "Source")?;
    let path = dir.path().join("a.recite");
    let changed = std::fs::read_to_string(&path)?.replace("Hello.", "Saved without closing.");
    fill(&mut test, "Hello.", &changed)?;
    support::click(&mut test, "Close a.recite")?;
    let action_id = |name| {
        test.find(|_, element| {
            let node = element.accessibility();
            (node.builder.role() == AccessibilityRole::Button && node.builder.label() == Some(name))
                .then_some(node.a11y_id)
                .flatten()
        })
    };
    let submit = action_id("Save and close document").ok_or("save close accessibility ID")?;
    let cancel = action_id("Cancel").ok_or("cancel accessibility ID")?;
    // Both real actions run in FIFO order before the root result task is polled.
    for id in [submit, cancel] {
        assert!(
            test.send_accessibility_action(AccessibilityActionEventData {
                action: AccessibilityAction::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: id,
                data: None,
            })
        );
    }
    for _ in 0..5_000 {
        test.sync_and_update();
        if std::fs::read_to_string(&path)? == changed && !has(&test, "Saving…") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(std::fs::read_to_string(path)?, changed);
    assert!(!has(&test, "Save and close document"));
    assert!(has(&test, "Saved without closing."));
    assert!(
        test.find(|_, element| Rect::try_downcast(element).filter(|r| r
            .accessibility
            .builder
            .label()
            == Some("Close a.recite")))
            .is_some()
    );
    Ok(())
}

#[test]
fn closing_a_saved_sibling_restores_applicable_staged_rules()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let original = ":: start default speaker=mara\n? reply@22222222222222222222 requires=(ready())\n  Continue through.\n  -> END\n";
    std::fs::write(dir.path().join("a.recite"), original)?;
    let mut test = open(dir.path())?;
    let field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Continue through.")))
                .map(|_| node.layout().area)
        })
        .ok_or("reply field")?;
    test.click_cursor(field.center().to_f64());
    test.poll_n(std::time::Duration::from_millis(16), 6);
    support::click(&mut test, "Line actions")?;
    support::wait_for(&mut test, "reply actions", |test| has(test, "Reply rules"))?;
    support::click(&mut test, "Reply rules")?;
    support::click(&mut test, "Make always available")?;
    support::click(&mut test, "B")?;
    support::click(&mut test, "Source")?;
    let other = std::fs::read_to_string(dir.path().join("b.recite"))?;
    fill(
        &mut test,
        "elsewhere",
        &format!("{other}\n# Changed sibling\n"),
    )?;
    support::click(&mut test, "Save")?;
    support::click(&mut test, "Close b.recite")?;
    assert!(has(&test, "Destination effects"));
    support::click(&mut test, "Apply rules")?;
    support::click(&mut test, "Edit in Source")?;
    assert!(has(&test, "Continue through."));
    assert!(!has(&test, "requires=(ready())"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        original
    );
    Ok(())
}
