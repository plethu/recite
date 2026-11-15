//! The continuous scene galley. The sheet shows the scene: its title once in Display, then every
//! beat in the galley window under a Section heading, 48px apart with a rule. Only the selected
//! beat mounts editable fields; the others are read-only text at the same metrics, so choosing a
//! beat never reflows the page. Each read-only beat's heading is a button that mounts it.
use super::passage::{grid, margin, margin_label, route_inset};
use super::*;
use crate::presentation::marks::{Mark, marked, wedge};
use crate::presentation::row::named_row;
use crate::presentation::sheet_text::sheet_text;
use crate::presentation::{Icon, print};
use crate::view::workspace::Grid;
use crate::view::workspace::grid::{DESTINATION_GUTTER, GUTTER};
use recite_writer_app::reading::PAGE;
use recite_writer_model::PassageKind;
use xilem::masonry::{
    core::{Property, UsesProperty},
    properties::LineBreaking,
    widgets::Flex,
};
use xilem::view::{FlexSpacer, sized_box};
#[cfg(test)]
mod tests;

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
        print::title(state.ui_scale(), scene)
            .prop(LineBreaking::WordWrap)
            .width((grid.row() - grid.text()).max(120.).px()),
    )
    .padding(Padding {
        left: grid.text().px(),
        bottom: space::GROUPS.px(),
        ..Padding::all(0.px())
    })
}

/// The 1px rule above every beat after the first, 24px either side of it.
fn rule(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    sized_box(
        flex_col(())
            .dims((grid.row().px(), 1.px()))
            .background_color(state.palette.rule),
    )
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
    let text = Type::Section
        .label(scale, name.clone())
        .color(if selected {
            state.palette.ink
        } else {
            state.palette.muted
        })
        .prop(LineBreaking::WordWrap);
    let title = if selected {
        // Padded like the read-only headings' buttons, so every heading starts on one line.
        Either::A(sized_box(text).padding(Padding::from_vh(0.px(), 13.px())))
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
            title.flex(1.),
            selected.then(|| actions(state, blocked)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap((grid.gutter() - 13.).max(0.).px())
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
            |state: &mut Writer| {
                state.open_project_workspace(crate::workspaces::Open::Rename);
            },
        ),
        print::icon_action(
            scale,
            "Pin reference",
            Icon::Pin,
            &state.palette,
            blocked,
            Writer::pin_reference,
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

/// A beat whose fields are not mounted: its heading button, then its rows as read-only text in
/// reading order.
fn read_only(
    state: &Writer,
    grid: &Grid,
    index: usize,
    total: usize,
    problems: &[(String, String)],
) -> impl WidgetView<Writer> + use<> {
    let rows = state.reading.beat_rows(index).cloned().unwrap_or_default();
    let shown = rows.len().min(PAGE);
    let more = rows.len() - shown;
    let ends = !matches!(
        rows.last().map(|row| &row.content),
        Some(Content::Continuation(_))
    );
    let lines = rows[..shown]
        .iter()
        .map(|row| read_only_row(state, grid, row))
        .collect::<Vec<_>>();
    flex_col((
        (index != state.reading.galley().start).then(|| rule(state, grid)),
        heading(state, grid, index, total, problems),
        sized_box(
            flex_col(lines)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::TEXT.px()),
        )
        .padding(Padding::top(space::KINDS.px())),
        (more > 0).then(|| {
            sized_box(
                print::meta(
                    state.ui_scale(),
                    format!(
                        "{more} more {} · edit the beat to read on",
                        if more == 1 { "entry" } else { "entries" }
                    ),
                )
                .color(state.palette.muted),
            )
            .padding(Padding::left(grid.text().px()))
        }),
        (ends && more == 0).then(|| {
            sized_box(
                divider(
                    Axis::Horizontal,
                    print::end_mark(state.ui_scale(), &state.palette),
                )
                .dash_pattern(&[1.px(), 3.px()])
                .prop(ContentColor::new(state.palette.rule))
                .width(grid.row().px()),
            )
            .padding(Padding::top(space::KINDS.px()))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(0.px())
}

/// One read-only row, on the same grid and at the same sizes as an editable one.
fn read_only_row(state: &Writer, grid: &Grid, row: &Row) -> Box<AnyWidgetView<Writer>> {
    let reading = state.reading_size();
    match &row.content {
        Content::Passage { passage, .. } => {
            let (label, number) = margin_label(passage, row.reply);
            // Replies take the 0.9R tier once fields can set their own leading; until then a smaller
            // reply would shrink its field's accessible line box below the reference.
            let size = Sheet::Dialogue.size(reading);
            let measure = grid.measure;
            let destination = match &passage.kind {
                PassageKind::Choice {
                    destination: Some(target),
                } if target != "END" => Some(format!("→ {}", display_name(target))),
                _ => None,
            };
            let wide = grid.margin > 0.;
            let route = |target: String| {
                Sheet::Destination
                    .label(reading, target)
                    .color(state.palette.teal_text)
                    .prop(LineBreaking::WordWrap)
            };
            let column = grid.destination > 0.;
            flex_col((
                flex_row((
                    wide.then(|| margin(state, grid, label, number)),
                    // Fields keep 2px of padding and border before their text; so does this.
                    sized_box(sheet_text(passage.text.clone(), size, state.palette.ink))
                        .padding(Padding::all(2.px()))
                        .width(measure.px()),
                    column.then(|| destination.clone()).flatten().map(|target| {
                        sized_box(route(target).width(grid.destination.px())).padding(
                            Padding::left(
                                (DESTINATION_GUTTER - GUTTER)
                                    .max(route_inset(state.ui_scale()))
                                    .px(),
                            ),
                        )
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
                .gap(grid.gutter().px()),
                (!column).then_some(destination).flatten().map(|target| {
                    sized_box(route(target).width(grid.measure.px()))
                        .padding(Padding::left((grid.text() + 2.).px()))
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::UNIT.px())
            .padding(Padding::top(
                if row.reply == Some(1) {
                    space::TEXT
                } else {
                    0.
                }
                .px(),
            ))
            .boxed()
        }
        Content::Heading(text) => sized_box(
            sheet_text(
                text.clone(),
                Sheet::Dialogue.size(reading),
                state.palette.muted,
            )
            .width(grid.measure.px()),
        )
        .padding(Padding::left(grid.text().px()))
        .boxed(),
        Content::Note(text) => sized_box(
            print::meta(state.ui_scale(), text.clone())
                .color(state.palette.muted)
                .prop(LineBreaking::WordWrap)
                .width(grid.measure.px()),
        )
        .padding(Padding::left(grid.text().px()))
        .boxed(),
        Content::Continuation(target) if target == "END" => {
            sized_box(print::end_mark(state.ui_scale(), &state.palette))
                .padding(Padding::left(grid.text().px()))
                .boxed()
        }
        Content::Continuation(target) => sized_box(
            Sheet::Destination
                .label(reading, format!("→ {}", display_name(target)))
                .color(state.palette.teal_text),
        )
        .padding(Padding::left(grid.text().px()))
        .boxed(),
    }
}
