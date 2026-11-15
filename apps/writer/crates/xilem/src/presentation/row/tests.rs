use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::widgets::Passthrough;

struct State {
    selected: bool,
    presses: usize,
}

fn view(state: &mut State) -> impl WidgetView<State, Widget = Passthrough> + use<> {
    crate::keyed::keyed(
        (),
        row(
            state.selected,
            label(1., "Relay Desk"),
            |state: &mut State| {
                state.presses += 1;
                state.selected = !state.selected;
            },
        ),
    )
}

#[test]
fn a_row_is_a_named_button_whose_selection_is_a_state_not_part_of_its_name() {
    let mut mounted = Mounted::new(
        State {
            selected: true,
            presses: 0,
        },
        view,
    );
    assert!(mounted.is_selected("Relay Desk"));
    mounted.activate("Relay Desk");
    assert_eq!(mounted.state.presses, 1);
    assert!(!mounted.is_selected("Relay Desk"));
}
