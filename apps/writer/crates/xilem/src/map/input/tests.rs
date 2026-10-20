use super::*;
use xilem::masonry::{
    core::{
        PointerButton, PointerEvent, PointerGesture, PointerGestureEvent, PointerId, PointerInfo,
        PointerState, PointerType,
    },
    dpi::PhysicalPosition,
    testing::TestHarness,
    theme,
    widgets::Label,
};

fn viewport() -> TestHarness<InputWidget> {
    TestHarness::create_with_size(
        theme::default_property_set(),
        NewWidget::new(InputWidget::new(
            NewWidget::new(Label::new("Map")),
            Kind::Viewport(Detail::Overview),
            false,
        )),
        (300, 200),
    )
}
fn actions(harness: &mut TestHarness<InputWidget>) -> Vec<Input> {
    let mut events = vec![];
    while let Some((action, _)) = harness.pop_action::<NativeAction>() {
        if let NativeAction::Input(input) = action {
            events.push(input);
        }
    }
    events
}
#[test]
fn camera_gestures_retire_the_press_before_release_can_activate_an_overview_beat() {
    for pinch in [false, true] {
        let mut harness = viewport();
        harness.mouse_move(Point::new(20., 20.));
        harness.mouse_button_press(Some(PointerButton::Primary));
        assert!(
            actions(&mut harness)
                .iter()
                .any(|input| matches!(input, Input::Begin(_)))
        );
        if pinch {
            harness.process_pointer_event(PointerEvent::Gesture(PointerGestureEvent {
                pointer: PointerInfo {
                    pointer_id: Some(PointerId::PRIMARY),
                    pointer_type: PointerType::Mouse,
                    persistent_device_id: None,
                },
                gesture: PointerGesture::Pinch(-0.1),
                state: PointerState {
                    position: PhysicalPosition::new(20., 20.),
                    ..PointerState::default()
                },
            }));
        } else {
            harness.mouse_wheel(Vec2::new(0., 20.));
        }
        assert!(harness.root_widget().press.is_none());
        harness.mouse_button_release(Some(PointerButton::Primary));
        let events = actions(&mut harness);
        assert!(events.iter().any(|input| matches!(input, Input::Cancel)));
        assert!(
            !events
                .iter()
                .any(|input| matches!(input, Input::OverviewClick(_) | Input::Finish))
        );
    }
}
#[test]
fn a_secondary_release_does_not_complete_a_primary_press() {
    let mut harness = viewport();
    harness.mouse_move(Point::new(20., 20.));
    harness.mouse_button_press(Some(PointerButton::Primary));
    actions(&mut harness);
    harness.mouse_button_release(Some(PointerButton::Secondary));
    assert!(actions(&mut harness).is_empty());
    assert!(harness.root_widget().press.is_some());
    harness.mouse_button_release(Some(PointerButton::Primary));
    assert!(
        actions(&mut harness)
            .iter()
            .any(|input| matches!(input, Input::Finish))
    );
}
