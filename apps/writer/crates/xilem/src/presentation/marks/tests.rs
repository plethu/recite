use super::wedge::{SIDE, Wedge, WedgePress};
use super::*;
use xilem::masonry::{
    accesskit::{Action, ActionRequest, Role, TreeId},
    core::NewWidget,
    testing::TestHarness,
    theme::default_property_set,
};

#[test]
fn the_error_wedge_is_a_24px_named_target_carrying_its_problem_as_the_description() {
    let wedge = NewWidget::new(Wedge::new(
        "Problem in Relay Desk".into(),
        "unknown block reference `nowhere`".into(),
        (Color::from_rgb8(137, 51, 48), Color::from_rgb8(104, 62, 98)),
    ));
    let mut harness = TestHarness::create_with_size(default_property_set(), wedge, (100, 100));
    let id = harness.root_id();
    harness.redraw();
    let node = harness.access_node(id).expect("wedge node");
    assert_eq!(node.role(), Role::Button);
    assert_eq!(node.label().as_deref(), Some("Problem in Relay Desk"));
    assert_eq!(
        node.description().as_deref(),
        Some("unknown block reference `nowhere`")
    );
    let size = harness.get_widget_with_id(id).ctx().border_box().size();
    assert!(size.width >= SIDE && size.height >= SIDE);
    harness.process_access_event(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: id.to_raw().into(),
        data: None,
    });
    assert_eq!(harness.pop_action::<WedgePress>(), Some((WedgePress, id)));
}

#[test]
fn current_and_unsaved_marks_differ_in_shape_as_well_as_ink() {
    let palette = super::super::palette(&recite_config::WriterConfig::default());
    let current = Mark::Current.bar(&palette);
    let unsaved = Mark::Unsaved.bar(&palette);
    assert_ne!(current.screen, unsaved.screen);
    assert_ne!(current.color, unsaved.color);
    assert_eq!(current.thickness, THICKNESS);
}
