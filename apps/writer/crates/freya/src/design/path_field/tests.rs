use super::*;
use freya_testing::prelude::*;
fn form() -> impl IntoElement {
    use_init_theme(light_theme);
    let value = use_state(String::new);
    let mut submitted = use_state(String::new);
    let id = use_a11y();
    let browse_id = use_a11y();
    use_after_side_effect(move || id.request_focus());
    rect()
        .width(Size::fill())
        .child(PathField {
            value,
            id,
            browse_id,
            kind: PathKind::Project,
            enabled: true,
            submit: EventHandler::new(move |()| submitted.set(value.peek().clone())),
        })
        .child(label().text(format!("Opened: {}", submitted.read())))
}
#[test]
fn manual_path_remains_editable_and_enter_submits() {
    let mut test = TestingRunner::new(form, Size2D::new(700., 300.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 4);
    test.write_text("/tmp/story/recite.project.toml");
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(
        test.find(|_, e| Label::try_downcast(e)
            .filter(|l| l.text.as_ref() == "Opened: /tmp/story/recite.project.toml"))
            .is_some()
    );
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.label() == Some("Browse for project folder")))
            .is_some()
    );
}
