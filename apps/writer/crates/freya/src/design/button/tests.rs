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
