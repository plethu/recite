use super::*;
use crate::app::keyboard::{FallbackFocus, ReturnFocus};
use xilem::masonry::core::CollectionWidget;

/// Focus never falls out of the window: when the focused control goes away, as Continue after a
/// preview step or the line under a view switch, its marked successor takes focus, and without
/// one the marked fallback, such as the chosen view segment.
#[test]
fn focus_that_loses_its_control_moves_to_the_marked_successor_before_the_fallback() {
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let card = root.get_widget_with_tag(MAP).unwrap().id();
    let result = root.get_widget_with_tag(RESULT).unwrap().id();
    root.edit_widget(result, |mut widget| widget.insert_prop(ReturnFocus(true)));
    keyboard.synchronize(&mut root);
    assert!(root.focused_widget().is_some(), "the window holds focus");
    root.focus_on(Some(card));
    // The card's place now holds the chosen segment; the card itself is gone.
    let segment = NewWidget::new(Button::with_text("Map")).with_props(FallbackFocus(true));
    let segment_id = segment.id();
    let place = root.get_layer_root(0).children()[0].children()[0].children()[1].id();
    root.edit_widget(place, |mut widget| {
        SizedBox::set_child(&mut widget.downcast::<SizedBox>(), segment);
    });
    assert_eq!(root.focused_widget(), None, "the card took focus with it");
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(result), "the successor first");
    // The successor goes too: the fallback takes focus.
    let body = root.get_layer_root(0).children()[0].id();
    root.edit_widget(body, |mut widget| {
        Flex::remove(&mut widget.downcast::<Flex>(), 2);
    });
    assert_eq!(root.focused_widget(), None);
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(segment_id));
}
