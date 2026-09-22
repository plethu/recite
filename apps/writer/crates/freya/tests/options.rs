mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn key(test: &mut TestingRunner, key: NamedKey, code: Code) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(key),
        code,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
}

#[test]
fn options_select_explicitly_and_arrow_keys_keep_tab_order_in_the_dialog()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut test, platform) = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1000., 800.),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Theme: Dark")?;
    let dark = *platform.focused_accessibility_id.peek();
    support::click(&mut test, "Theme: Dark")?;
    assert_eq!(*platform.focused_accessibility_id.peek(), dark);
    key(&mut test, NamedKey::ArrowLeft, Code::ArrowLeft);
    assert_ne!(*platform.focused_accessibility_id.peek(), dark);
    let light = test
        .find(|_, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Theme: Light"))
                .map(|r| r.accessibility.builder.toggled())
        })
        .ok_or("Light option")?;
    assert_eq!(light, Some(accesskit::Toggled::True));
    key(&mut test, NamedKey::Tab, Code::Tab);
    let keymap = *platform.focused_accessibility_id.peek();
    support::click(&mut test, "Keymap: Standard")?;
    assert_eq!(*platform.focused_accessibility_id.peek(), keymap);
    key(&mut test, NamedKey::ArrowRight, Code::ArrowRight);
    key(&mut test, NamedKey::Tab, Code::Tab);
    let view = *platform.focused_accessibility_id.peek();
    let dialog = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.role() == AccessibilityRole::Dialog)
                .map(|_| node.layout().area)
        })
        .ok_or("settings dialog")?;
    let map = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility.builder.label() == Some("Writing view: Map")
                        && dialog.contains(node.layout().area.center())
                })
                .map(|_| node.layout().area)
        })
        .ok_or("Map preference inside Settings")?;
    test.click_cursor((f64::from(map.center().x), f64::from(map.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert_ne!(*platform.focused_accessibility_id.peek(), view);
    assert!(
        test.find(|node, e| {
            Rect::try_downcast(e).filter(|r| {
                r.accessibility.builder.label() == Some("Writing view: Map")
                    && dialog.contains(node.layout().area.center())
                    && r.accessibility.builder.toggled() == Some(accesskit::Toggled::True)
            })
        })
        .is_some()
    );
    Ok(())
}

#[test]
fn scene_disclosures_contain_beats_and_switching_preserves_the_accordion()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 900.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 4);
    let has_group = |test: &TestingRunner, name: &str| {
        test.find(|_, e| {
            Rect::try_downcast(e).filter(|r| {
                r.accessibility.builder.label() == Some(name)
                    && r.accessibility.builder.is_expanded() == Some(true)
            })
        })
        .is_some()
    };
    assert!(has_group(&test, "Relay Hub"));
    support::click(&mut test, "Relay Hub")?;
    assert!(!has_group(&test, "Relay Hub"));
    support::click(&mut test, "Floodgate Waterfall")?;
    assert!(has_group(&test, "Floodgate Waterfall"));
    support::click(&mut test, "Relay Hub")?;
    assert!(has_group(&test, "Relay Hub"));
    assert!(!has_group(&test, "Floodgate Waterfall"));
    Ok(())
}
