use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::{accesskit::Role, widgets::Passthrough};
use xilem::view::button;

struct State(bool);

fn view(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    crate::keyed::keyed(
        (),
        button(
            disclosure(1., state.0, Color::BLACK, label(1., "Relay Hub")),
            |state: &mut State| state.0 = !state.0,
        ),
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
fn a_painted_chevron_keeps_the_announced_disclosure_glyph() {
    let mut mounted = Mounted::new(State(false), view);
    assert_eq!(names(&mounted), ["▸ Relay Hub"]);
    mounted.activate("▸ Relay Hub");
    assert_eq!(names(&mounted), ["▾ Relay Hub"]);
}
