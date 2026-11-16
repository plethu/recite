//! The continuous scene galley. The sheet shows the scene: its title once in Display, then every
//! beat in the galley window under a Section heading, 48px apart with a rule. Only the selected
//! beat mounts editable fields; the others are read-only text at the same metrics, so choosing a
//! beat never reflows the page. Each read-only beat's heading is a button that mounts it.
use super::passage::{grid, margin, margin_label};
use super::*;
use crate::app::keyboard::SlipAnchor;
use crate::presentation::control_inset;
use crate::presentation::grid::{DESTINATION_GUTTER, GUTTER, Grid};
use crate::presentation::marks::{Mark, marked, wedge};
use crate::presentation::row::named_row;
use crate::presentation::sheet_text::sheet_text;
use crate::presentation::tokens::size;
use crate::presentation::{Icon, print};
use crate::workspaces::rename::inline::{self as rename, Place};
use recite_writer_app::reading::PAGE;
use recite_writer_model::PassageKind;
use xilem::masonry::{
    core::{Property, UsesProperty},
    properties::LineBreaking,
    widgets::Flex,
};
use xilem::view::{FlexSpacer, sized_box};
mod read_only;
#[cfg(test)]
mod tests;
use read_only::read_only;

/// Marks the selected beat's heading, so the native host can bring the selection into view.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct SelectedBeat(pub bool);
impl Property for SelectedBeat {
    fn static_default() -> &'static Self {
        static OTHER: SelectedBeat = SelectedBeat(false);
        &OTHER
    }
}
impl UsesProperty<SelectedBeat> for Flex {}

/// Half the space between beats: the rule sits midway in the 48px.
const HALF_BEAT: f64 = space::BEATS / 2.;

/// The galley around the selected beat, whose own rows are `body`.
pub(super) fn galley<V: WidgetView<Writer>>(
    state: &Writer,
    body: V,
) -> impl WidgetView<Writer> + use<V> {
    let grid = grid(state);
    let range = state.reading.galley();
    let selected = state
        .reading
        .position
        .as_ref()
        .map(|position| position.ordinal() - 1);
    let total = state.reading.script().map_or(0, <[_]>::len);
    let problems = state.beat_problems();
    let before = range
        .clone()
        .filter(|index| selected.is_some_and(|selected| *index < selected))
        .map(|index| read_only(state, &grid, index, total, &problems).boxed())
        .collect::<Vec<_>>();
    let after = range
        .filter(|index| selected.is_none_or(|selected| *index > selected))
        .map(|index| read_only(state, &grid, index, total, &problems).boxed())
        .collect::<Vec<_>>();
    let first = before.is_empty();
    flex_col((
        scene_title(state, &grid),
        flex_col(before).gap(0.px()),
        selected.map(|index| {
            flex_col((
                (!first).then(|| rule(state, &grid)),
                heading(state, &grid, index, total, &problems),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(0.px())
        }),
        body,
        flex_col(after).gap(0.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(0.px())
}

/// The scene's title in Display, once at the top of the sheet.
fn scene_title(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    let key = state.session.model().document().key().as_str();
    let scene = display_name(key.rsplit('/').next().unwrap_or(key));
    sized_box(
        print::title(Sheet::scale(state.reading_size()), scene)
            .prop(LineBreaking::WordWrap)
            .width((grid.row() - grid.text()).px()),
    )
    .padding(Padding {
        left: grid.text().px(),
        bottom: space::GROUPS.px(),
        ..Padding::all(0.px())
    })
}

/// The 1px rule above every beat after the first, 24px either side of it.
fn rule(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    sized_box(print::hairline(&state.palette))
        .width(grid.row().px())
        .padding(Padding::from_vh(HALF_BEAT.px(), 0.px()))
}

fn beat_id(state: &Writer, index: usize) -> String {
    state
        .reading
        .script()
        .ok()
        .and_then(|blocks| blocks.get(index))
        .map_or_else(String::new, |block| block.id.clone())
}

/// A beat's heading row: margin marks, the Section heading, and for the selected beat its
/// actions at the end of the row.
fn heading(
    state: &Writer,
    grid: &Grid,
    index: usize,
    total: usize,
    problems: &[(String, String)],
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let id = beat_id(state, index);
    let name = display_name(&id);
    let selected = state.reading.selected.as_deref() == Some(id.as_str());
    let blocked = state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy();
    let problem = problems
        .iter()
        .find(|(beat, _)| beat == &id)
        .map(|(_, message)| message.clone());
    let inset = control_inset(scale);
    // Set with the manuscript: a Section heading at the reading size, whatever the UI scale.
    let sheet = Sheet::scale(state.reading_size());
    let text = Type::Section
        .label(sheet, name.clone())
        .color(if selected {
            state.palette.ink
        } else {
            state.palette.muted
        })
        .prop(LineBreaking::WordWrap);
    // The field gives way to the heading once the review is ready, so focus moves to its slip.
    let renaming = rename::editing(state, &id, Place::Heading).filter(|_| selected);
    let renaming_here = renaming.is_some();
    let title = if let Some(inline) = renaming {
        Either::A(Either::A(rename::field(
            state,
            inline,
            Type::Section,
            sheet,
        )))
    } else if selected {
        // Padded like the read-only headings' buttons, so every heading starts on one line.
        Either::A(Either::B(
            sized_box(text).padding(Padding::from_vh(0.px(), inset.px())),
        ))
    } else {
        Either::B(styled(
            ControlStyle::Action,
            named_row(
                format!("Edit beat {name}, {} of {total}", index + 1),
                text,
                move |state: &mut Writer| state.edit_beat(index),
            )
            .disabled(blocked),
        ))
    };
    marked(
        &state.palette,
        selected.then_some(Mark::Current),
        flex_row((
            flex_row((
                FlexSpacer::Flex(1.),
                problem.map(|message| {
                    wedge(
                        format!("Problem in {name}"),
                        message,
                        (state.palette.error, state.palette.accent),
                        move |state: &mut Writer| state.edit_beat(index),
                    )
                }),
            ))
            .width(grid.margin.px()),
            // The heading hugs its words, so a focused heading's ring stays inside the measure. A
            // rename's slip floats over the window from under it, so nothing below moves.
            sized_box(flex_col((title,)).cross_axis_alignment(if renaming_here {
                // The rename field takes the heading's measure.
                CrossAxisAlignment::Stretch
            } else {
                CrossAxisAlignment::Start
            }))
            .prop(SlipAnchor(
                selected && rename::hangs_from(state, &id, Place::Heading),
            ))
            .flex(1.),
            selected.then(|| actions(state, blocked)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap((grid.gutter() - inset).max(0.).px())
        .width(grid.row().px())
        .prop(SelectedBeat(selected)),
    )
}

/// The selected beat's quiet icon actions, always in the Tab order.
fn actions(state: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::icon_action(
            scale,
            "Rename beat",
            Icon::Rename,
            &state.palette,
            blocked || state.session.project().is_none(),
            Writer::rename_beat,
        ),
        print::icon_toggle(
            scale,
            ("Pin reference", Icon::Pin),
            &state.palette,
            (state.reference_pinned(), blocked),
            Writer::toggle_reference,
        ),
        print::icon_action(
            scale,
            "Copy link",
            Icon::Link,
            &state.palette,
            blocked,
            Writer::copy_link,
        ),
    ))
    .gap(space::UNIT.px())
}
