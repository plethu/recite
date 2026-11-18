use super::*;
use crate::presentation::label;
use crate::presentation::marks::underscored;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::{core::WidgetId, kurbo::Rect, layout::AsUnit, widgets::Passthrough};
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, flex_col};

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

struct Pair {
    first: bool,
}

/// Two rows with the same words, one selected, each underscored while selected.
fn pair(state: &mut Pair) -> impl WidgetView<Pair, Widget = Passthrough> + use<> {
    let palette = crate::presentation::palette(&recite_config::WriterConfig::default());
    let entry = |selected: bool, name: &'static str| {
        listed(
            selected,
            row(
                selected,
                underscored(&palette, selected, 1., label(1., "Relay Desk")),
                move |state: &mut Pair| state.first = name == "First",
            )
            .accessible_label(name),
        )
    };
    crate::keyed::keyed(
        (),
        flex_col((entry(state.first, "First"), entry(!state.first, "Second")))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(0.px()),
    )
}

/// The window rectangle of `id`'s border box.
fn bounds(
    mounted: &Mounted<Pair, impl WidgetView<Pair, Widget = Passthrough>>,
    id: WidgetId,
) -> Rect {
    let widget = mounted.harness.get_widget_with_id(id);
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
}

/// The innermost first descendant of `id`: a row's words.
fn words(
    mounted: &Mounted<Pair, impl WidgetView<Pair, Widget = Passthrough>>,
    id: WidgetId,
) -> WidgetId {
    let mut widget = mounted.harness.get_widget_with_id(id);
    while let Some(child) = widget.children().first().cloned() {
        widget = child;
    }
    widget.id()
}

#[test]
fn selection_underscores_the_words_in_ink_without_moving_the_row_or_its_text() {
    let mut mounted = Mounted::new(Pair { first: true }, pair);
    let ink = crate::presentation::palette(&recite_config::WriterConfig::default())
        .ink
        .to_rgba8()
        .to_u8_array();
    let rows = ["First", "Second"].map(|name| mounted.find(name));
    let [selected, other] = rows.map(|id| bounds(&mounted, id));
    let [selected_words, other_words] = rows.map(|id| bounds(&mounted, words(&mounted, id)));
    // The same box and the same text origin, selected or not.
    assert_eq!(selected.size(), other.size());
    assert_eq!(
        selected_words.origin() - selected.origin(),
        other_words.origin() - other.origin()
    );
    assert!(mounted.is_selected("First") && !mounted.is_selected("Second"));
    let image = mounted.harness.render();
    let under = |words: Rect, x: f64| image.get_pixel(x as u32, (words.y1 + 1.) as u32).0;
    // Ink under the selected row's words, inside the row; nothing past them or under the other.
    assert_eq!(under(selected_words, selected_words.x0 + 2.), ink);
    assert!(selected_words.y1 + 2. <= selected.y1);
    assert_ne!(under(selected_words, selected.x1 - 2.), ink);
    assert_ne!(under(other_words, other_words.x0 + 2.), ink);
    // Selection moves with a press; the underscore follows it and nothing else changes.
    mounted.activate("Second");
    assert!(mounted.is_selected("Second") && !mounted.is_selected("First"));
    assert_eq!(bounds(&mounted, rows[0]).size(), selected.size());
}
