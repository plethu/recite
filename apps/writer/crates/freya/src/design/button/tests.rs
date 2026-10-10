use super::*;
use crate::design::{ReducedMotion, palette};
use freya_testing::prelude::*;
use std::time::Duration;

fn specimen() -> impl IntoElement {
    use_init_theme(|| palette::theme(true));
    let reduced = use_state(|| false);
    use_provide_context(|| ReducedMotion(reduced));
    let mut count = use_state(|| 0);
    rect()
        .child(
            Button::new()
                .named("Action")
                .on_press(move |_| {
                    let next = *count.peek() + 1;
                    count.set(next);
                })
                .child("Action"),
        )
        .child(Button::new().flat().named("Quiet").child("Quiet"))
        .child(Button::new().filled().named("Primary").child("Primary"))
        .child(
            Button::new()
                .enabled(false)
                .named("Disabled")
                .child("Disabled"),
        )
        .child(label().text(format!("Calls: {}", count.read())))
}

#[test]
fn action_heights_match_and_quiet_hover_fades_without_blackening() {
    let mut test = TestingRunner::new(specimen, Size2D::new(300., 240.), |_| {}, 1.).0;
    test.poll_n(Duration::from_millis(16), 4);
    let area = |test: &TestingRunner, name: &str| {
        test.find(|node, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some(name))
                .map(|_| node.layout().area)
        })
        .expect("action")
    };
    for name in ["Action", "Quiet", "Primary", "Disabled"] {
        assert_eq!(area(&test, name).height(), 32.);
    }
    test.move_cursor(area(&test, "Quiet").center().to_f64());
    test.sync_and_update();
    test.poll_n(Duration::from_millis(16), 2);
    let color = test
        .find(|_, element| {
            Rect::try_downcast(element)
                .filter(|r| r.accessibility.builder.label() == Some("Quiet"))
                .and_then(|r| r.style.background.as_color())
        })
        .expect("hover fill");
    let hover = palette::Palette::new(true).hover;
    assert!(color.a() > 0 && color.a() < 255, "{color:?}");
    assert_eq!(
        (color.r(), color.g(), color.b()),
        (hover.r(), hover.g(), hover.b())
    );
}

fn surface(test: &TestingRunner) -> (Area, Color) {
    test.find(|node, element| {
        Rect::try_downcast(element)
            .filter(|rect| rect.accessibility.builder.label() == Some("Action"))
            .map(|rect| {
                (
                    node.layout().area,
                    rect.style.background.as_color().expect("flat button fill"),
                )
            })
    })
    .expect("action")
}

#[test]
fn pointer_and_keyboard_show_pressed_state_without_moving_the_target_or_double_activation() {
    let mut test = TestingRunner::new(specimen, Size2D::new(300., 120.), |_| {}, 1.).0;
    test.poll_n(Duration::from_millis(16), 4);
    let text_y = |test: &TestingRunner| {
        test.find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text == "Action")
                .map(|_| node.layout().area.min_y())
        })
        .expect("button caption")
    };
    let resting_text = text_y(&test);
    let (area, resting) = surface(&test);
    test.press_cursor(area.center().to_f64());
    test.poll_n(Duration::from_millis(16), 6);
    let (held_area, held) = surface(&test);
    assert_eq!(area, held_area);
    assert_ne!(resting, held);
    assert!((text_y(&test) - resting_text - 1.).abs() < 0.01);
    test.release_cursor(area.center().to_f64());
    test.move_cursor(CursorPoint::new(299., 119.));
    test.poll_n(Duration::from_millis(16), 12);
    assert_eq!(resting, surface(&test).1);
    assert!((text_y(&test) - resting_text).abs() < 0.01);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(Duration::from_millis(16), 6);
    assert_eq!(held, surface(&test).1);
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyUp,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(Duration::from_millis(16), 12);
    assert_eq!((area, resting), surface(&test));
    assert!(
        test.find(
            |_, element| Label::try_downcast(element).filter(|label| label.text == "Calls: 2")
        )
        .is_some()
    );
}

#[test]
fn press_depth_preserves_content_names_and_explicit_accessible_names() {
    let (mut test, platform) = TestingRunner::new(
        || {
            rect()
                .child(Button::new().child("Cancel"))
                .child(Button::new().named("Save all documents").child("Save all"))
        },
        (300., 200.).into(),
        |r| r.provide_root_context(Platform::get),
        1.,
    );
    test.poll_n(Duration::from_millis(16), 4);
    for name in ["Cancel", "Save all documents"] {
        let area = test
            .find(|node, element| {
                Rect::try_downcast(element)
                    .filter(|r| r.accessibility.builder.label() == Some(name))
                    .map(|_| node.layout().area)
            })
            .expect("named button");
        test.click_cursor(area.center().to_f64());
        test.poll_n(Duration::from_millis(16), 4);
        assert_eq!(
            platform.focused_accessibility_node.peek().label(),
            Some(name)
        );
    }
}

fn accessible_node(button: Button) -> accesskit::Node {
    button
        .with_accessibility(rect())
        .get_accessibility_data()
        .builder
        .clone()
}

#[test]
fn each_role_exposes_only_its_applicable_state() {
    for button in [Button::new(), Button::new().menu_item()] {
        let node = accessible_node(button);
        assert_eq!(node.is_selected(), None);
        assert_eq!(node.toggled(), None);
    }
    for state in [false, true] {
        for (button, role) in [
            (Button::new().tab(state), AccessibilityRole::Tab),
            (
                Button::new().option(state),
                AccessibilityRole::ListBoxOption,
            ),
        ] {
            let node = accessible_node(button);
            assert_eq!(node.role(), role);
            assert_eq!(node.is_selected(), Some(state));
            assert_eq!(node.toggled(), None);
        }
        for (button, role) in [
            (Button::new().toggle(state), AccessibilityRole::Button),
            (Button::new().checkable(state), AccessibilityRole::CheckBox),
            (Button::new().radio(state), AccessibilityRole::RadioButton),
        ] {
            let node = accessible_node(button);
            assert_eq!(node.role(), role);
            assert_eq!(node.is_selected(), None);
            assert_eq!(
                node.toggled(),
                Some(if state {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                })
            );
        }
    }
}

#[test]
fn changing_roles_cannot_retain_a_previous_roles_state() {
    let radio = accessible_node(Button::new().checkable(true).radio(false));
    assert_eq!(radio.role(), AccessibilityRole::RadioButton);
    assert_eq!(radio.toggled(), Some(accesskit::Toggled::False));
    assert_eq!(radio.is_selected(), None);

    for button in [
        Button::new().radio(true).menu_item(),
        Button::new().tab(true).menu_item(),
    ] {
        let menu_item = accessible_node(button);
        assert_eq!(menu_item.role(), AccessibilityRole::MenuItem);
        assert_eq!(menu_item.toggled(), None);
        assert_eq!(menu_item.is_selected(), None);
    }
    let option = accessible_node(Button::new().checkable(true).option(false));
    assert_eq!(option.role(), AccessibilityRole::ListBoxOption);
    assert_eq!(option.is_selected(), Some(false));
    assert_eq!(option.toggled(), None);
}

#[test]
fn visual_highlighting_does_not_claim_or_override_accessible_state() {
    let action = accessible_node(Button::new().highlighted(true));
    assert_eq!(action.role(), AccessibilityRole::Button);
    assert_eq!(action.toggled(), None);
    assert_eq!(action.is_selected(), None);

    let checkbox = accessible_node(Button::new().checkable(false).highlighted(true));
    assert_eq!(checkbox.role(), AccessibilityRole::CheckBox);
    assert_eq!(checkbox.toggled(), Some(accesskit::Toggled::False));
    assert_eq!(checkbox.is_selected(), None);
}

#[test]
fn common_accessibility_properties_remain_independent_of_the_role() {
    let mut control = Button::new()
        .checkable(false)
        .enabled(false)
        .expanded(false)
        .shortcut("Ctrl+K".into())
        .named("Explicit name")
        .child(label().text("Visible caption"))
        .with_accessibility(rect());
    let accessibility = control.get_accessibility_data();
    let node = &accessibility.builder;
    assert_eq!(node.role(), AccessibilityRole::CheckBox);
    assert!(node.is_disabled());
    assert_eq!(accessibility.a11y_focusable, false.into());
    assert_eq!(node.is_expanded(), Some(false));
    assert_eq!(node.keyboard_shortcut(), Some("Ctrl+K"));
    assert_eq!(node.label(), Some("Explicit name"));

    let action = accessible_node(
        Button::new()
            .shortcut(String::new())
            .child(label().text("Caption")),
    );
    assert!(!action.is_disabled());
    assert_eq!(action.is_expanded(), None);
    assert_eq!(action.keyboard_shortcut(), None);
    assert_eq!(action.label(), Some("Caption"));
}
