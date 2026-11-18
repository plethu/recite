use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::{accesskit::Role, widgets::Passthrough};
use xilem::view::{CrossAxisAlignment, button, flex_row};

struct State(bool);

fn view(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    crate::keyed::keyed(
        (),
        button(
            disclosure(1., state.0, Color::BLACK, label(1., "Relay Hub")),
            |state: &mut State| state.0 = !state.0,
        )
        .expanded(state.0),
    )
}

fn names(mounted: &Mounted<State, impl WidgetView<State, Widget = Passthrough>>) -> Vec<String> {
    let tree = mounted.harness.access_tree();
    let mut names = Vec::new();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.role() == Role::Button {
            names.extend(node.label());
        }
        pending.extend(node.children());
    }
    names
}

#[test]
fn a_disclosure_reports_expanded_and_keeps_its_name_across_states() {
    let mut mounted = Mounted::new(State(false), view);
    assert_eq!(names(&mounted), ["Relay Hub"]);
    assert!(mounted.is_expanded("Relay Hub", false));
    mounted.activate("Relay Hub");
    assert_eq!(names(&mounted), ["Relay Hub"]);
    assert!(mounted.is_expanded("Relay Hub", true));
}

fn route_row(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    crate::keyed::keyed(
        (),
        flex_row((
            label(1., "Reply"),
            button(
                route_toggle((1., 2.), state.0, Color::BLACK, 14., label(1., "Route")),
                |state: &mut State| state.0 = !state.0,
            )
            .accessible_label("→ Route"),
        ))
        .cross_axis_alignment(CrossAxisAlignment::FirstBaseline),
    )
}

/// The top of each label's words, by text.
fn tops(mounted: &Mounted<State, impl WidgetView<State, Widget = Passthrough>>) -> [f64; 2] {
    let tree = mounted.harness.access_tree();
    let mut found = Vec::new();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.role() == Role::Label {
            found.push((node.value(), node.bounding_box().unwrap().y0));
        }
        pending.extend(node.children());
    }
    ["Reply", "Route"].map(|text| {
        found
            .iter()
            .find(|(value, _)| value.as_deref() == Some(text))
            .unwrap_or_else(|| panic!("{text} in {found:?}"))
            .1
    })
}

#[test]
fn a_route_toggle_sets_its_words_on_the_row_baseline_open_or_closed() {
    let mut mounted = Mounted::new(State(false), route_row);
    let [reply, route] = tops(&mounted);
    assert!((reply - route).abs() < 0.5, "closed: {reply} {route}");
    mounted.activate("→ Route");
    let [reply, route] = tops(&mounted);
    assert!((reply - route).abs() < 0.5, "open: {reply} {route}");
}
