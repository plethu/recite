mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn primary() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}
fn primary_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Rect::try_downcast(e).filter(|r| r.accessibility.builder.label() == Some(text))
    })
    .is_some()
}
fn key(test: &mut TestingRunner, key: Key, code: Code, modifiers: Modifiers) {
    for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
        test.send_event(PlatformEvent::Keyboard {
            name,
            key: key.clone(),
            code,
            modifiers,
        });
        test.poll_n(Duration::from_millis(16), 4);
    }
}
fn open(test: &mut TestingRunner) -> Result {
    support::click(test, "Settings")?;
    support::click(test, "Keyboard shortcuts")?;
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&dir)?;
        test.render_to_file(std::path::Path::new(&dir).join("bindings-list.png"));
    }
    support::click(
        test,
        &format!("Rebind Commands: {}+Shift+P", primary_name()),
    )
}
#[test]
fn conflicting_binding_requires_replacement_and_cancel_preserves_original() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 1000.).into(), |_| {}, 1.).0;
    open(&mut test)?;
    key(&mut test, Key::Character("s".into()), Code::KeyS, primary());
    assert!(has(&test, "Replace binding"));
    key(
        &mut test,
        Key::Named(NamedKey::Escape),
        Code::Escape,
        Modifiers::empty(),
    );
    assert!(has(&test, "Done"));
    let commands = format!("Rebind Commands: {}+Shift+P", primary_name());
    assert!(has(&test, &commands));
    support::click(&mut test, &commands)?;
    key(&mut test, Key::Character("s".into()), Code::KeyS, primary());
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        test.render_to_file(std::path::Path::new(&dir).join("binding-conflict.png"));
    }
    support::click(&mut test, "Replace binding")?;
    assert!(has(
        &test,
        &format!("Rebind Commands: {}+S", primary_name())
    ));
    assert!(has(&test, "Rebind Save: Unassigned"));
    Ok(())
}
#[test]
fn modifiers_can_be_chosen_one_at_a_time() -> Result {
    let (mut test, _platform) = TestingRunner::new(
        recite_writer::app,
        (1200., 1000.).into(),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    open(&mut test)?;
    support::click(&mut test, "Choose keys separately")?;
    support::click(&mut test, primary_name())?;
    support::click(&mut test, "Shift")?;
    support::click(&mut test, "Press keys…")?;
    key(
        &mut test,
        Key::Character("k".into()),
        Code::KeyK,
        Modifiers::empty(),
    );
    if let Ok(dir) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        test.render_to_file(std::path::Path::new(&dir).join("binding-separate-keys.png"));
    }
    support::click(&mut test, "Save binding")?;
    assert!(has(
        &test,
        &format!("Rebind Commands: {}+Shift+K", primary_name())
    ));
    Ok(())
}
#[test]
fn recording_reserved_keys_keeps_settings_open_and_does_not_change_binding() -> Result {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 1000.).into(), |_| {}, 1.).0;
    open(&mut test)?;
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        primary(),
    );
    assert!(has(&test, "Back to shortcuts"));
    key(
        &mut test,
        Key::Named(NamedKey::F6),
        Code::F6,
        Modifiers::empty(),
    );
    assert!(has(&test, "Back to shortcuts"));
    support::click(&mut test, "Back to shortcuts")?;
    assert!(has(
        &test,
        &format!("Rebind Commands: {}+Shift+P", primary_name())
    ));
    Ok(())
}

#[test]
fn binding_can_be_changed_entirely_with_keyboard_and_focus_returns_to_row() -> Result {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1200., 1000.).into(),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Keyboard shortcuts")?;
    for _ in 0..6 {
        key(
            &mut test,
            Key::Named(NamedKey::Tab),
            Code::Tab,
            Modifiers::empty(),
        );
    }
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has(&test, "Back to shortcuts"));
    key(
        &mut test,
        Key::Named(NamedKey::F8),
        Code::F8,
        Modifiers::empty(),
    );
    for _ in 0..2 {
        key(
            &mut test,
            Key::Named(NamedKey::Tab),
            Code::Tab,
            Modifiers::empty(),
        );
    }
    key(
        &mut test,
        Key::Named(NamedKey::Enter),
        Code::Enter,
        Modifiers::empty(),
    );
    assert!(has(&test, "Rebind Commands: F8"));
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("Rebind Commands: F8")
    );
    Ok(())
}
