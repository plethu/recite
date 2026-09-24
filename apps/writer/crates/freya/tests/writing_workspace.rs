mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn named(test: &TestingRunner, name: &str) -> Option<Area> {
    test.find(|node, e| {
        Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some(name))
            .map(|_| node.layout().area)
    })
}
fn has_map(test: &TestingRunner) -> bool {
    test.find(|_, e| {
        Rect::try_downcast(e).filter(|r| {
            r.accessibility
                .builder
                .label()
                .is_some_and(|l| l.starts_with("Scene map."))
        })
    })
    .is_some()
}
#[test]
fn standalone_script_split_and_focus_restore_the_workspace() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1440., 1000.).into(), |_| {}, 1.).0;
    assert!(named(&test, "Beat editor").is_some());
    assert!(!has_map(&test));
    assert!(named(&test, "Script + Map").is_none());
    assert!(named(&test, "Focus writing").is_none());
    support::click(&mut test, "Workspace")?;
    support::click(&mut test, "Script + Map")?;
    assert!(has_map(&test));
    let before = named(&test, "Beat editor").ok_or("split editor")?;
    support::click(&mut test, "Workspace")?;
    support::click(&mut test, "Focus writing")?;
    assert!(!has_map(&test));
    assert!(named(&test, "Hide scenes").is_none());
    support::click(&mut test, "Exit focus writing")?;
    assert!(has_map(&test));
    assert_eq!(named(&test, "Beat editor"), Some(before));
    Ok(())
}
#[test]
fn map_selection_does_not_open_until_requested() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1440., 1000.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Map")?;
    let card = named(&test, "Select Relay Desk").ok_or("card")?;
    test.click_cursor((card.center().x as f64, (card.min_y() + 20.) as f64));
    test.sync_and_update();
    assert!(named(&test, "Beat editor").is_none());
    support::click(&mut test, "Open script")?;
    assert!(named(&test, "Beat editor").is_some());
    Ok(())
}
#[test]
fn command_search_navigates_and_cancels_without_changing_view() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Commands")?;
    test.write_text("source");
    test.poll_n(std::time::Duration::from_millis(16), 5);
    test.press_key(Key::Named(NamedKey::ArrowDown));
    test.press_key(Key::Named(NamedKey::Enter));
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(named(&test, "Source editor").is_some());
    support::click(&mut test, "Commands")?;
    test.press_key(Key::Named(NamedKey::Escape));
    test.sync_and_update();
    assert!(named(&test, "Source editor").is_some());
    Ok(())
}
#[test]
fn capture_reading_sizes_and_constrained_workspace() -> Result {
    for (width, height) in [(900., 650.), (1280., 800.), (1600., 1000.)] {
        let mut test = TestingRunner::new(recite_writer::app, (width, height).into(), |_| {}, 1.).0;
        test.poll_n(std::time::Duration::from_millis(16), 15);
        assert!(named(&test, "Beat editor").is_some());
        if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
            std::fs::create_dir_all(&dir)?;
            test.render_to_file(std::path::Path::new(&dir).join(format!("script-{width}.png")));
        }
        support::click(&mut test, "Settings")?;
        support::click(&mut test, "Increase Reading size")?;
        support::click(&mut test, "Done")?;
        test.poll_n(std::time::Duration::from_millis(16), 15);
        assert!(named(&test, "Beat editor").is_some());
    }
    Ok(())
}

#[test]
fn large_text_keeps_primary_controls_reachable() -> Result {
    use recite_config::{
        Platform as ConfigPlatform, PlatformRoots, UserConfigStore, resolve_config_path,
    };
    for width in [900., 1280., 1600.] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.toml");
        std::fs::write(
            &path,
            "config_version = 1\n[writer]\ntheme = \"dark\"\n[writer.presentation]\nreading_size = 32\nsource_size = 28\nui_scale = 200\nsplit = true\n",
        )?;
        let store = UserConfigStore::new(resolve_config_path(
            ConfigPlatform::Linux,
            &PlatformRoots::new(),
            Some(&path),
        )?);
        let mut test = TestingRunner::new(
            recite_writer::app,
            (width, 800.).into(),
            move |r| r.provide_root_context(move || Ok::<_, String>(store)),
            1.,
        )
        .0;
        test.poll_n(std::time::Duration::from_millis(16), 15);
        for name in ["Commands", "Writing view: Source", "Workspace"] {
            let area = named(&test, name).ok_or(name)?;
            assert!(
                area.min_x() >= 0. && area.max_x() <= width && area.max_y() <= 800.,
                "{name}: {area:?}"
            );
        }
        support::click(&mut test, "Workspace")?;
        for name in ["Focus writing", "Localise", "Save project"] {
            let area = named(&test, name).ok_or(name)?;
            assert!(
                area.min_x() >= 0. && area.max_x() <= width,
                "{name}: {area:?}"
            );
        }
        test.press_key(Key::Named(NamedKey::Escape));
        test.sync_and_update();
        assert_eq!(has_map(&test), width >= 1600.);
        if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
            std::fs::create_dir_all(&dir)?;
            test.render_to_file(
                std::path::Path::new(&dir).join(format!("script-large-dark-{width}.png")),
            );
        }
        support::click(&mut test, "Settings")?;
        let done = named(&test, "Done").ok_or("Done")?;
        assert!(done.min_y() >= 0. && done.max_y() <= 800.);
        support::click(&mut test, "Done")?;
        support::click(&mut test, "Source")?;
        test.poll_n(std::time::Duration::from_millis(16), 8);
        for name in ["Source editor", "Apply draft", "Discard draft"] {
            let area = named(&test, name).ok_or(name)?;
            assert!(
                area.min_x() >= 0. && area.max_x() <= width && area.max_y() <= 800.,
                "{name}: {area:?}"
            );
        }
        if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
            test.render_to_file(
                std::path::Path::new(&dir).join(format!("source-large-dark-{width}.png")),
            );
        }
    }
    Ok(())
}

#[test]
fn returning_to_a_scene_preserves_its_source_draft_and_view() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1440., 1000.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Source editor")?;
    test.press_key(Key::Named(NamedKey::End));
    test.write_text(" // retained");
    support::click(&mut test, "Last Tram Linear")?;
    support::click(&mut test, "Script")?;
    support::click(&mut test, "Relay Hub")?;
    assert!(named(&test, "Source editor").is_some());
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("retained"))))
            .is_some()
    );
    Ok(())
}

#[test]
fn keyboard_opens_selected_map_card_and_go_to_finds_a_beat() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1440., 1000.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Map")?;
    support::click(&mut test, "Select Relay Desk")?;
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(named(&test, "Beat editor").is_some());
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("p".into()),
        code: Code::KeyP,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.poll_n(std::time::Duration::from_millis(16), 6);
    test.write_text("Missing Courier");
    test.poll_n(std::time::Duration::from_millis(16), 6);
    test.press_key(Key::Named(NamedKey::ArrowDown));
    test.press_key(Key::Named(NamedKey::Enter));
    test.poll_n(std::time::Duration::from_millis(16), 6);
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("Our courier is two days late"))))
            .is_some()
    );
    Ok(())
}

#[test]
fn leaving_the_map_preserves_a_deliberately_panned_camera() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1440., 1000.).into(), |_| {}, 1.).0;
    support::click(&mut test, "Map")?;
    let card = named(&test, "Select Relay Desk").ok_or("card")?;
    test.scroll(
        (card.center().x as f64, card.center().y as f64),
        (30., -20.),
    );
    test.sync_and_update();
    let panned = named(&test, "Select Relay Desk").ok_or("panned card")?;
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Map")?;
    assert_eq!(named(&test, "Select Relay Desk"), Some(panned));
    Ok(())
}

#[test]
fn compact_workspace_menu_supports_keyboard_and_mode_changes() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (900., 650.).into(), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    support::click(&mut test, "Workspace")?;
    test.press_key(Key::Named(NamedKey::End));
    test.poll_n(std::time::Duration::from_millis(16), 3);
    test.press_key(Key::Named(NamedKey::Enter));
    test.sync_and_update();
    assert!(named(&test, "Done").is_some());
    support::click(&mut test, "Done")?;
    support::click(&mut test, "Workspace")?;
    support::click(&mut test, "Localise")?;
    let has_catalogue_action = |test: &TestingRunner| {
        test.find(|_, e| Label::try_downcast(e).filter(|label| label.text == "Open PO catalogue"))
            .is_some()
    };
    assert!(has_catalogue_action(&test));
    support::click(&mut test, "Workspace")?;
    support::click(&mut test, "Script")?;
    assert!(named(&test, "Beat editor").is_some());
    assert!(!has_catalogue_action(&test));
    Ok(())
}
