mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn key(test: &mut TestingRunner, code: Code, key: Key, modifiers: Modifiers) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        code,
        key,
        modifiers,
    });
    test.poll_n(std::time::Duration::from_millis(16), 8);
}
fn area(test: &TestingRunner, name: &str) -> Option<Area> {
    test.find(|node, e| {
        Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some(name))
            .map(|_| node.layout().area)
    })
}
#[test]
fn keyboard_and_vim_share_selection_editing_and_focus_return()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1440., 1000.),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keymap: Vim")?;
    support::click(&mut test, "Done")?;
    support::click(&mut test, "Map")?;
    // Map opens with graph focus. Cycle through the drawer back to the graph.
    key(
        &mut test,
        Code::F6,
        Key::Named(NamedKey::F6),
        Modifiers::empty(),
    );
    // j selects below the entry and Enter opens its editor.
    key(
        &mut test,
        Code::F6,
        Key::Named(NamedKey::F6),
        Modifiers::empty(),
    );
    key(
        &mut test,
        Code::KeyJ,
        Key::Character("j".into()),
        Modifiers::empty(),
    );
    let selected = test.find(|_, e| {
        Rect::try_downcast(e).filter(|r| {
            r.accessibility
                .builder
                .label()
                .is_some_and(|l| l.starts_with("Scene map. Selected Station History"))
        })
    });
    assert!(
        selected.is_some(),
        "Down selects the nearest beat in the next row"
    );
    key(
        &mut test,
        Code::Enter,
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
    );
    assert!(area(&test, "Beat editor").is_some());
    key(
        &mut test,
        Code::Escape,
        Key::Named(NamedKey::Escape),
        Modifiers::empty(),
    );
    assert!(area(&test, "Beat editor").is_none());
    assert_ne!(platform.focused_accessibility_id.peek().0, 0);
    Ok(())
}
#[test]
fn drawer_toggle_keeps_its_vertical_position_and_source_uses_full_height()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1440., 1000.), |_| {}, 1.).0;
    let before = area(&test, "Hide scenes").ok_or("hide")?;
    support::click(&mut test, "Hide scenes")?;
    let after = area(&test, "Show scenes").ok_or("show")?;
    assert_eq!(before.min_y(), after.min_y());
    assert_eq!(before.height(), after.height());
    support::click(&mut test, "Show scenes")?;
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Floodgate Waterfall")?;
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == "Find a beat…"))
            .is_none()
    );
    let source = area(&test, "Source editor").ok_or("source editor")?;
    assert!(source.height() > 750., "{source:?}");
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("source-full-height.png"));
    }
    Ok(())
}

#[test]
fn personal_preferences_survive_a_new_window_and_settings_keep_focus()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{
        Platform as ConfigPlatform, PlatformRoots, UserConfigStore, WriterView, resolve_config_path,
    };
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "config_version = 1\n")?;
    let store = UserConfigStore::new(resolve_config_path(
        ConfigPlatform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    let context = store.clone();
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1440., 1000.),
        move |r| {
            let _ = r.provide_root_context(move || Ok::<_, String>(context));
            r.provide_root_context(Platform::get)
        },
        1.,
    );
    support::click(&mut test, "Source")?;
    assert_eq!(store.load()?.config.writer.view, WriterView::Source);
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Theme: Dark")?;
    let focused = *platform.focused_accessibility_id.peek();
    key(
        &mut test,
        Code::F6,
        Key::Named(NamedKey::F6),
        Modifiers::empty(),
    );
    assert_eq!(*platform.focused_accessibility_id.peek(), focused);
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("settings-dark.png"));
    }
    drop(test);
    let (test, _) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1440., 1000.),
        move |r| r.provide_root_context(move || Ok::<_, String>(store)),
        1.,
    );
    assert!(area(&test, "Source editor").is_some());
    Ok(())
}
