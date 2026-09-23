//! Repeatable accessibility contracts, not native screen-reader acceptance.
use freya::prelude::*;
use freya_testing::prelude::*;
use recite_config::{
    Platform as ConfigPlatform, PlatformRoots, UserConfigStore, resolve_config_path,
};
use std::{collections::BTreeSet, time::Duration};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

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
    test.poll_n(Duration::from_millis(16), 4);
}

fn named_key(test: &mut TestingRunner, key_value: NamedKey, code: Code) {
    key(test, Key::Named(key_value), code, Modifiers::empty());
}

fn command(test: &mut TestingRunner, name: &str) {
    key(
        test,
        Key::Character("P".into()),
        Code::KeyP,
        Modifiers::SHIFT
            | if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            },
    );
    test.write_text(name);
    test.poll_n(Duration::from_millis(16), 4);
    named_key(test, NamedKey::Enter, Code::Enter);
}

fn tab_to(test: &mut TestingRunner, platform: &Platform, name: &str) {
    for _ in 0..100 {
        if platform.focused_accessibility_node.peek().label() == Some(name) {
            return;
        }
        named_key(test, NamedKey::Tab, Code::Tab);
    }
    panic!("Tab could not reach {name}");
}

fn dialog(test: &TestingRunner) -> Result<Area> {
    test.find(|node, e| {
        let data = e.accessibility();
        (data.builder.role() == AccessibilityRole::Dialog).then(|| {
            assert!(data.builder.is_modal(), "dialog must be marked modal");
            assert!(data.builder.label().is_some_and(|s| !s.trim().is_empty()));
            node.layout().area
        })
    })
    .ok_or_else(|| "open dialog".into())
}

fn audit_buttons(test: &TestingRunner) {
    let controls = test.find_many(|node, element| {
        let data = element.accessibility();
        let role = data.builder.role();
        if !matches!(
            role,
            AccessibilityRole::Button
                | AccessibilityRole::CheckBox
                | AccessibilityRole::RadioButton
        ) {
            return None;
        }
        assert!(
            data.builder.label().is_some_and(|s| !s.trim().is_empty()),
            "unnamed mounted {role:?} at {:?}",
            node.layout().area
        );
        let area = node.layout().area;
        assert!(area.width() > 0. && area.height() > 0., "empty {role:?}");
        Some(())
    });
    assert!(!controls.is_empty(), "audit must inspect controls");
}

#[test]
fn settings_keyboard_cycle_stays_visible_and_returns_focus_at_large_sizes() -> Result {
    for scale in [100, 200] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("config.toml");
        std::fs::write(
            &path,
            format!(
                "config_version = 1\n[writer]\nreduced_motion = true\n[writer.presentation]\nui_scale = {scale}\n"
            ),
        )?;
        let store = UserConfigStore::new(resolve_config_path(
            ConfigPlatform::Linux,
            &PlatformRoots::new(),
            Some(&path),
        )?);
        let (mut test, platform) = TestingRunner::new(
            recite_writer::app,
            (900., 650.).into(),
            move |runner| {
                let _store = runner.provide_root_context(move || Ok::<_, String>(store));
                runner.provide_root_context(Platform::get)
            },
            1.,
        );
        audit_buttons(&test);
        named_key(&mut test, NamedKey::F6, Code::F6);
        let origin = *platform.focused_accessibility_id.peek();
        command(&mut test, "User preferences");
        let mut seen = BTreeSet::new();
        for _ in 0..100 {
            let id = *platform.focused_accessibility_id.peek();
            if !seen.insert(id) {
                break;
            }
            let focused = platform.focused_accessibility_node.peek().clone();
            let name = focused.label().expect("focused control name");
            assert!(!name.trim().is_empty());
            let bounds = focused.bounds().expect("focused control bounds");
            let surface = dialog(&test)?;
            assert!(
                bounds.x0 >= f64::from(surface.min_x()) - 1.
                    && bounds.x1 <= f64::from(surface.max_x()) + 1.
                    && bounds.y0 >= f64::from(surface.min_y()) - 1.
                    && bounds.y1 <= f64::from(surface.max_y()) + 1.,
                "{scale}%: {name} is outside dialog: {bounds:?}, {surface:?}"
            );
            named_key(&mut test, NamedKey::Tab, Code::Tab);
        }
        assert!(
            seen.len() >= 12,
            "must traverse settings, not a tiny focus loop"
        );
        key(
            &mut test,
            Key::Named(NamedKey::Tab),
            Code::Tab,
            Modifiers::SHIFT,
        );
        assert!(seen.contains(&*platform.focused_accessibility_id.peek()));
        tab_to(&mut test, &platform, "Keyboard shortcuts");
        named_key(&mut test, NamedKey::Enter, Code::Enter);
        audit_buttons(&test);
        tab_to(
            &mut test,
            &platform,
            if cfg!(target_os = "macos") {
                "Rebind Commands: Cmd+Shift+P"
            } else {
                "Rebind Commands: Ctrl+Shift+P"
            },
        );
        named_key(&mut test, NamedKey::Enter, Code::Enter);
        named_key(&mut test, NamedKey::Escape, Code::Escape);
        assert!(
            platform
                .focused_accessibility_node
                .peek()
                .label()
                .is_some_and(|s| s.starts_with("Rebind Commands:"))
        );
        named_key(&mut test, NamedKey::Escape, Code::Escape);
        assert_eq!(*platform.focused_accessibility_id.peek(), origin);
        audit_buttons(&test);
    }
    Ok(())
}

#[test]
fn workspace_screens_keep_mounted_buttons_named() {
    let mut test = TestingRunner::new(recite_writer::app, (1200., 900.).into(), |_| {}, 1.).0;
    for screen in ["Map", "Source", "Script", "User preferences"] {
        command(&mut test, screen);
        audit_buttons(&test);
    }
}

#[test]
fn settings_from_workspace_menu_returns_to_the_menu_trigger() -> Result {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1200., 900.).into(),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    tab_to(&mut test, &platform, "Workspace");
    let origin = *platform.focused_accessibility_id.peek();
    named_key(&mut test, NamedKey::Enter, Code::Enter);
    named_key(&mut test, NamedKey::End, Code::End);
    assert_eq!(
        platform.focused_accessibility_node.peek().label(),
        Some("User preferences")
    );
    named_key(&mut test, NamedKey::Enter, Code::Enter);
    dialog(&test)?;
    named_key(&mut test, NamedKey::Escape, Code::Escape);
    assert_eq!(*platform.focused_accessibility_id.peek(), origin);
    Ok(())
}

#[test]
fn source_can_be_replaced_applied_and_read_without_a_pointer() {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        (1200., 900.).into(),
        |runner| runner.provide_root_context(Platform::get),
        1.,
    );
    command(&mut test, "Source");
    for _ in 0..4 {
        named_key(&mut test, NamedKey::F6, Code::F6);
        if platform.focused_accessibility_node.peek().role() == AccessibilityRole::TextInput {
            break;
        }
    }
    assert_eq!(
        platform.focused_accessibility_node.peek().role(),
        AccessibilityRole::TextInput
    );
    key(
        &mut test,
        Key::Character("a".into()),
        Code::KeyA,
        if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    );
    test.write_text(
        ":: start\n> greeting@11111111111111111111\n  Keyboard-only writing.\n-> END\n",
    );
    test.poll_n(Duration::from_millis(16), 4);
    command(&mut test, "Apply");
    command(&mut test, "Script");
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("Keyboard-only writing."))))
            .is_some(),
        "the applied source must be readable in Script"
    );
}
