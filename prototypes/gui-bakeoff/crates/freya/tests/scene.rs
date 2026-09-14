use freya::prelude::*;
use freya_testing::prelude::*;

fn click_label(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == caption)
                .map(|_| node.layout().area)
        })
        .ok_or("label missing")?;
    assert!(
        area.min_x() >= 0. && area.max_x() <= 1200. && area.min_y() >= 0. && area.max_y() <= 800.,
        "{caption} is outside the window: {area:?}"
    );
    assert!(
        area.height() < 40.,
        "{caption} is squeezed into a vertical label"
    );
    test.move_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.sync_and_update();
    assert_eq!(test.cursor_icon(), CursorIcon::Pointer, "{caption} cursor");
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}

#[test]
fn draft_refusal_apply_and_source_script_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|paragraph| {
                    paragraph
                        .spans
                        .iter()
                        .any(|span| span.text.contains("Would you tell me"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("prose input missing")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("A different question.");
    click_label(&mut test, "Source")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("Apply or discard")))
            .is_some()
    );
    click_label(&mut test, "Apply draft")?;
    click_label(&mut test, "Source")?;
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text.contains("A different question.")))
        )
        .is_some()
    );
    click_label(&mut test, "Script")?;
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text == "A different question."))
        )
        .is_some()
    );
    click_label(&mut test, "Undo")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Would you tell me"))))
            .is_some()
    );
    click_label(&mut test, "Redo")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "A different question.")))
            .is_some()
    );
    Ok(())
}

#[test]
fn whole_scene_source_and_dark_theme_render() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("That depends a good deal")))
            .is_some()
    );
    capture(&mut test, "script-light.png")?;
    click_label(&mut test, "Try scene")?;
    capture(&mut test, "script-preview.png")?;
    click_label(&mut test, "Continue")?;
    click_label(&mut test, "Close preview")?;
    click_label(&mut test, "Dark")?;
    capture(&mut test, "script-dark.png")?;
    click_label(&mut test, "Source")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == "Light"))
            .is_some()
    );
    test.poll_n(std::time::Duration::from_millis(16), 4);
    capture(&mut test, "source-dark.png")?;
    click_label(&mut test, "Light")?;
    capture(&mut test, "source-light.png")?;
    Ok(())
}

fn capture(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(directory) = std::env::var("RECITE_BAKEOFF_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.poll_n(std::time::Duration::from_millis(16), 4);
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
    Ok(())
}

#[test]
fn project_open_save_and_conflict_keep_the_authored_text() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    std::fs::write(
        dir.path().join("second.recite"),
        ":: other\n> other@33333333333333333333\n  Other scene.\n-> END\n",
    )?;
    let source_path = dir.path().join("scene.recite");
    std::fs::write(&source_path, recite_bakeoff_authoring::FIXTURE)?;
    let (mut test, platform) = TestingRunner::new(
        recite_bakeoff_freya::editor_app,
        Size2D::new(1200., 800.),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    let path_field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Project folder")))
                .map(|_| node.layout().area)
        })
        .ok_or("project path field")?;
    test.click_cursor((
        f64::from(path_field.min_x() + 2.),
        f64::from(path_field.min_y() + 2.),
    ));
    test.write_text(dir.path().to_string_lossy());
    click_label(&mut test, "Open project")?;
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Would you tell me")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("A saved question.");
    let previous_focus = *platform.focused_accessibility_id.peek();
    assert_eq!(
        recite_bakeoff_freya::request_close(),
        CloseDecision::KeepOpen
    );
    test.poll_n(std::time::Duration::from_millis(16), 4);
    capture(&mut test, "writer-close.png")?;
    let initial = *platform.focused_accessibility_id.peek();
    assert_ne!(initial, previous_focus);
    for _ in 0..3 {
        test.send_event(PlatformEvent::Keyboard {
            name: KeyboardEventName::KeyDown,
            key: Key::Named(NamedKey::Tab),
            code: Code::Tab,
            modifiers: Modifiers::empty(),
        });
        test.poll_n(std::time::Duration::from_millis(16), 4);
        assert_ne!(*platform.focused_accessibility_id.peek(), previous_focus);
    }
    assert_eq!(*platform.focused_accessibility_id.peek(), initial);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Escape),
        code: Code::Escape,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_eq!(*platform.focused_accessibility_id.peek(), previous_focus);
    capture(&mut test, "writer-project.png")?;
    let recovery_path = dir.path().join("scene.recite.recite-editor-recovery.json");
    assert!(std::fs::read_to_string(&recovery_path)?.contains("A saved question."));
    click_label(&mut test, "Second")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|l| l.text.contains("before changing files")))
            .is_some()
    );
    assert_eq!(
        std::fs::read_to_string(&source_path)?,
        recite_bakeoff_authoring::FIXTURE
    );
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("s".into()),
        code: Code::KeyS,
        modifiers: Modifiers::CONTROL,
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(!recovery_path.exists());
    assert_eq!(recite_bakeoff_freya::request_close(), CloseDecision::Close);
    assert!(std::fs::read_to_string(&source_path)?.contains("A saved question."));
    assert!(std::fs::read_to_string(&source_path)?.contains("7701ceab59d2adfa057a"));
    std::fs::write(&source_path, "external edit")?;
    click_label(&mut test, "Save")?;
    assert_eq!(std::fs::read_to_string(&source_path)?, "external edit");
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|l| l.text.contains("changed on disk")))
            .is_some()
    );
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "A saved question.")))
            .is_some()
    );
    Ok(())
}

#[test]
fn line_details_can_be_opened_and_closed_without_panicking()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    click_label(&mut test, "Passage actions ▾")?;
    click_label(&mut test, "Line / choice details")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("7701ceab59d2adfa057a")))
            .is_some()
    );
    click_label(&mut test, "Passage actions ▾")?;
    click_label(&mut test, "Line / choice details")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("7701ceab59d2adfa057a")))
            .is_none()
    );
    Ok(())
}

#[test]
fn folding_and_menu_dismissal_preserve_the_draft() -> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    let field_area = |test: &TestingRunner| {
        test.find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Would you tell me")))
                .map(|_| node.layout().area)
        })
        .expect("prose field")
    };
    let initial = field_area(&test);
    click_label(&mut test, "Hide scenes")?;
    assert!(field_area(&test).width() > initial.width());
    let area = field_area(&test);
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text("Draft ");
    click_label(&mut test, "▾ Which Way")?;
    assert!(
        test.find(|_, el| Paragraph::try_downcast(el)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Draft"))))
            .is_none()
    );
    capture(&mut test, "script-folded.png")?;
    click_label(&mut test, "▸ Which Way")?;
    assert!(
        test.find(|_, el| Paragraph::try_downcast(el)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Draft"))))
            .is_some()
    );
    let before_menu = field_area(&test);
    click_label(&mut test, "Passage actions ▾")?;
    assert_eq!(
        field_area(&test),
        before_menu,
        "popover must not displace the script"
    );
    let first = *platform.focused_accessibility_id.peek();
    capture(&mut test, "passage-menu.png")?;
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::ArrowDown),
        code: Code::ArrowDown,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_ne!(*platform.focused_accessibility_id.peek(), first);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Escape),
        code: Code::Escape,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(
        test.find(|_, el| Label::try_downcast(el).filter(|l| l.text.as_ref() == "Add choice"))
            .is_none()
    );
    let trigger = *platform.focused_accessibility_id.peek();
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_ne!(*platform.focused_accessibility_id.peek(), trigger);
    test.click_cursor((10., 790.));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(
        test.find(|_, el| Label::try_downcast(el).filter(|l| l.text.as_ref() == "Add choice"))
            .is_none()
    );
    click_label(&mut test, "Show scenes")?;
    Ok(())
}
