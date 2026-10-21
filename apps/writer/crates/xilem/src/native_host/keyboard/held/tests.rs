use super::*;
use xilem::winit::keyboard::{KeyCode, PhysicalKey};

#[test]
fn consumed_press_owns_repeats_and_one_release_independently_of_focus_or_scope() {
    let mut held = HeldKeys::default();
    let enter = PhysicalKey::Code(KeyCode::Enter);
    let space = PhysicalKey::Code(KeyCode::Space);
    assert!(!held.consume(enter, ElementState::Released));
    held.press(enter);
    held.press(enter);
    assert!(held.consume(enter, ElementState::Pressed));
    assert!(!held.consume(space, ElementState::Released));
    held.press(space);
    assert!(held.consume(enter, ElementState::Released));
    assert!(!held.consume(enter, ElementState::Released));
    assert!(held.consume(space, ElementState::Released));
}
