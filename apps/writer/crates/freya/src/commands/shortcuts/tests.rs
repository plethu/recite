use super::*;
#[test]
fn bindings_match_layout_characters_and_shifted_base_digits_without_stealing_plain_typing() {
    let event = |key: &str, code, modifiers| {
        KeyboardEventData::new(Key::Character(key.into()), code, modifiers)
    };
    assert!(matches_binding(
        "Primary+Shift+1",
        &event("!", Code::Digit1, primary() | Modifiers::SHIFT)
    ));
    assert!(matches_binding(
        "Primary+Shift+,",
        &event("<", Code::Comma, primary() | Modifiers::SHIFT)
    ));
    assert!(matches_binding(
        "Primary+P",
        &event("p", Code::KeyP, primary())
    ));
    assert!(!matches_binding(
        "Primary+P",
        &event("p", Code::KeyP, Modifiers::empty())
    ));
    assert!(!matches_binding(
        "Primary+P",
        &event("p", Code::KeyP, primary() | Modifiers::SHIFT)
    ));
    assert!(!matches_binding("", &event("p", Code::KeyP, primary())));
}
