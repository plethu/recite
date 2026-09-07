use masonry::{
    core::{Ime, NewWidget, TextEvent},
    kurbo::Size,
    testing::TestHarness,
    theme::default_property_set,
    widgets::{TextAction, TextArea},
};

#[test]
fn composition_is_not_a_committed_edit_and_unicode_commits() {
    let area = NewWidget::new(TextArea::new_editable(""));
    let mut harness =
        TestHarness::create_with_size(default_property_set(), area, Size::new(220., 130.));
    let id = harness.root_widget().id();
    harness.focus_on(Some(id));
    harness.process_text_event(TextEvent::Ime(Ime::Enabled));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit("にほん".into(), Some((9, 9)))));
    if let Some((TextAction::Changed(text), _)) = harness.pop_action::<TextAction>() {
        assert!(text.is_empty(), "preedit leaked into authored text: {text}");
    }
    harness.process_text_event(TextEvent::Ime(Ime::Commit("日本\nCafé 🐈".into())));
    let action = harness.pop_action::<TextAction>();
    assert!(matches!(action, Some((TextAction::Changed(text), _)) if text == "日本\nCafé 🐈"));
}
