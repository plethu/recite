use crate::navigation::tests::fixture;
use crate::test_support::finish_navigation;

#[test]
fn the_pin_toggle_pins_the_selected_beat_and_unpins_it_when_pressed_again() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    assert!(!writer.reference_pinned());
    writer.toggle_reference();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.reading_context.pinned.is_some());
    assert!(writer.reference_pinned());
    writer.toggle_reference();
    assert!(writer.reading_context.pinned.is_none());
    assert!(!writer.reference_pinned());
}

#[test]
fn a_different_selected_beat_is_not_the_pinned_reference() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.toggle_reference();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    writer.reading.selected = Some("second".into());
    assert!(!writer.reference_pinned());
}
