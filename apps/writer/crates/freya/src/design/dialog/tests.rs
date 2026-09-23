use super::*;
use crate::design::{SubmitAction, keyboard};
use freya_testing::prelude::*;

fn form() -> impl IntoElement {
    use_init_theme(light_theme);
    let query = use_state(String::new);
    let mut count = use_state(|| 0);
    let field = use_a11y();
    let button = use_a11y();
    Dialog {
        dismissal_only: false,
        title: "Test form".into(),
        reduced_motion: true,
        content: rect()
            .child(
                Input::new(query)
                    .auto_focus(true)
                    .a11y_id(field)
                    .on_pre_key_down(crate::closing::text_input_key),
            )
            .child(label().text(format!("Calls: {}", count.read())))
            .into_element(),
        actions: rect().into_element(),
        focus_order: vec![field, button],
        close: EventHandler::new(|()| {}),
        primary: SubmitAction {
            id: button,
            caption: "Submit".into(),
            enabled: !query.read().is_empty(),
            action: EventHandler::new(move |()| {
                let next = *count.peek() + 1;
                count.set(next);
            }),
        },
    }
}
fn calls(test: &TestingRunner, expected: usize) -> bool {
    test.find(|_, e| {
        Label::try_downcast(e).filter(|label| label.text.as_ref() == format!("Calls: {expected}"))
    })
    .is_some()
}
fn submit(test: &mut TestingRunner) {
    for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
        test.send_event(PlatformEvent::Keyboard {
            name,
            key: Key::Named(NamedKey::Enter),
            code: Code::Enter,
            modifiers: keyboard::primary_modifier(),
        });
        test.poll_n(std::time::Duration::from_millis(16), 3);
    }
}
#[test]
fn shortcut_respects_disabled_state_and_submits_once_from_input_or_button()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(form, Size2D::new(900., 650.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 4);
    submit(&mut test);
    assert!(calls(&test, 0));
    test.write_text("ready");
    submit(&mut test);
    assert!(calls(&test, 1));
    let area = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Submit"))
                .map(|_| node.layout().area)
        })
        .ok_or("submit button")?;
    test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(calls(&test, 2));
    submit(&mut test);
    assert!(calls(&test, 3));
    Ok(())
}

#[test]
fn dialog_contains_character_navigation_events() {
    fn app() -> impl IntoElement {
        let mut leaked = use_state(|| false);
        rect()
            .expanded()
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if keyboard::vim_commands_key(&event) {
                    leaked.set(true);
                }
            })
            .child(form())
            .child(
                rect()
                    .position(Position::new_global().top(0.).left(0.))
                    .child(label().text(format!("Leaked: {}", leaked.read()))),
            )
    }
    let mut test = TestingRunner::new(app, Size2D::new(900., 650.), |_| {}, 1.).0;
    test.write_text("ready");
    test.press_key(Key::Named(NamedKey::Tab));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    test.press_key(Key::Character(":".into()));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(
        test.find(|_, e| Label::try_downcast(e).filter(|l| l.text == "Leaked: false"))
            .is_some()
    );
}
