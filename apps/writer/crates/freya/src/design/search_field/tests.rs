use super::*;
use freya_testing::prelude::*;

fn form() -> impl IntoElement {
    use_init_theme(light_theme);
    let query = use_state(String::new);
    let active = use_state(|| None);
    let mut chosen = use_state(|| None::<usize>);
    let id = use_a11y();
    use_after_side_effect(move || id.request_focus());
    rect()
        .width(Size::fill())
        .child(SearchField {
            query,
            id,
            placeholder: "Find".into(),
            active,
            count: 3,
            vim: true,
            activate: EventHandler::new(move |index| chosen.set(Some(index))),
            changed: EventHandler::new(|()| {}),
        })
        .child(label().text(format!("Query: {}", query.read())))
        .child(label().text(format!("Active: {:?}", active.read())))
        .child(label().text(format!("Chosen: {:?}", chosen.read())))
}
fn key(test: &mut TestingRunner, key: Key) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key,
        code: Code::Unidentified,
        modifiers: Modifiers::empty(),
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
}
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == text))
        .is_some()
}
#[test]
fn text_entry_vim_navigation_activation_and_clear_share_state()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(form, Size2D::new(700., 400.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 4);
    test.write_text("jk");
    assert!(has(&test, "Query: jk"));
    key(&mut test, Key::Named(NamedKey::ArrowDown));
    assert!(has(&test, "Active: Some(0)"));
    key(&mut test, Key::Named(NamedKey::Escape));
    key(&mut test, Key::Character("j".into()));
    assert!(has(&test, "Active: Some(1)"));
    assert!(has(&test, "Query: jk"));
    key(&mut test, Key::Named(NamedKey::Enter));
    assert!(has(&test, "Chosen: Some(1)"));
    let area = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| r.accessibility.builder.label() == Some("Clear Find"))
                .map(|_| node.layout().area)
        })
        .ok_or("clear")?;
    assert!(area.min_y() >= t::SPACE_XS);
    assert!(area.max_y() <= 32. - t::SPACE_XS);
    assert!(area.max_x() <= 700. - t::SPACE_XS);
    test.click_cursor((f64::from(area.center().x), f64::from(area.center().y)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    assert!(has(&test, "Query: "));
    assert!(has(&test, "Active: None"));
    test.write_text("next");
    assert!(
        has(&test, "Query: next"),
        "query={:?}",
        test.find(|_, e| Label::try_downcast(e)
            .filter(|l| l.text.starts_with("Query:"))
            .map(|l| l.text.to_string()))
    );
    Ok(())
}
