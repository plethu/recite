//! Dynamic content must not leave End stuck behind a stale scrollbar position.
use masonry::{
    accesskit::Orientation,
    core::{
        KeyboardEvent, NewWidget, TextEvent,
        keyboard::{Key, KeyState, NamedKey},
    },
    layout::AsUnit,
    testing::TestHarness,
    widgets::{Portal, SizedBox},
};

fn end(harness: &mut TestHarness<Portal<SizedBox>>) {
    harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
        state: KeyState::Down,
        key: Key::Named(NamedKey::End),
        ..KeyboardEvent::default()
    }));
}
fn resize(harness: &mut TestHarness<Portal<SizedBox>>, size: f64) {
    harness.edit_root_widget(|mut root| {
        let mut child = Portal::child_mut(&mut root);
        SizedBox::set_size(&mut child, size.px(), size.px());
    });
    harness.redraw();
}

#[test]
fn end_tracks_the_viewport_after_shrink_and_regrowth_in_both_axes() {
    for shrunk in [100., 600.] {
        let content = NewWidget::new(SizedBox::empty().size(1000.px(), 1000.px()));
        let mut harness = TestHarness::create_with_size(
            masonry::theme::default_property_set(),
            NewWidget::new(Portal::new(content)),
            (240, 240),
        );
        let bars = [Orientation::Horizontal, Orientation::Vertical].map(|orientation| {
            harness
                .root_widget()
                .children()
                .into_iter()
                .find(|widget| {
                    harness
                        .access_node(widget.id())
                        .is_some_and(|node| node.orientation() == Some(orientation))
                })
                .unwrap()
                .id()
        });
        for id in bars {
            harness.focus_on(Some(id));
            end(&mut harness);
        }
        assert_eq!(
            harness.root_widget().get_viewport_pos(),
            masonry::kurbo::Point::new(760., 760.)
        );
        resize(&mut harness, shrunk);
        let clamped = (shrunk - 240.).max(0.);
        assert_eq!(
            harness.root_widget().get_viewport_pos(),
            masonry::kurbo::Point::new(clamped, clamped)
        );
        resize(&mut harness, 1200.);
        for id in bars {
            harness.focus_on(Some(id));
            end(&mut harness);
        }
        assert_eq!(
            harness.root_widget().get_viewport_pos(),
            masonry::kurbo::Point::new(960., 960.),
            "End must reach the new content edge after shrinking to {shrunk}"
        );
    }
}
