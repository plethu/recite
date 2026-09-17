use super::*;

#[test]
fn submit_requires_exact_platform_modifier_and_enter() {
    let event =
        |modifiers| KeyboardEventData::new(Key::Named(NamedKey::Enter), Code::Enter, modifiers);
    assert!(submit_key(&event(primary_modifier())));
    for modifiers in [
        Modifiers::empty(),
        Modifiers::ALT,
        primary_modifier() | Modifiers::SHIFT,
    ] {
        assert!(!submit_key(&event(modifiers)));
    }
    assert!(!submit_key(&KeyboardEventData::new(
        Key::Character("j".into()),
        Code::KeyJ,
        primary_modifier()
    )));
}

#[test]
fn vim_navigation_does_not_capture_insert_or_modified_characters() {
    for (letter, code, step) in [("j", Code::KeyJ, 1), ("k", Code::KeyK, -1)] {
        let mut event =
            KeyboardEventData::new(Key::Character(letter.into()), code, Modifiers::empty());
        assert_eq!(list_step(&event, false), None);
        assert_eq!(list_step(&event, true), Some(step));
        event.modifiers = Modifiers::CONTROL;
        assert_eq!(list_step(&event, true), None);
    }
}
