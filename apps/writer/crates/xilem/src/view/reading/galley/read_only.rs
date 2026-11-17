//! Beats whose fields are not mounted: their rows as read-only text in reading order, on the same
//! grid and at the same sizes as the editable beat, so choosing a beat never reflows the page.
use super::*;
use crate::view::reading::passage::{RouteFace, absorbed, continuation_row, joined};
use xilem::masonry::properties::Dimensions;
use xilem::view::{Label, MainAxisAlignment};

/// A beat whose fields are not mounted: its heading button, then its rows as read-only text in
/// reading order.
pub(super) fn read_only(
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
    let lines = (0..shown)
        // A continuation set on its line's row takes no row of its own.
        .filter(|&index| !absorbed(&rows, index))
        .map(|index| read_only_row(state, grid, &rows[index], joined(&rows, index)))
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
            sized_box(print::end_rule(
                state.ui_scale(),
                &state.palette,
                grid.row(),
            ))
            .padding(Padding::top(space::KINDS.px()))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(0.px())
}

/// One read-only row, on the same grid and at the same sizes as an editable one.
fn read_only_row(
    state: &Writer,
    grid: &Grid,
    row: &Row,
    joined: Option<&str>,
) -> Box<AnyWidgetView<Writer>> {
    let page = Page::of(state);
    match &row.content {
        Content::Passage { passage, .. } => {
            let (label, number) = margin_label(passage, row.reply);
            let size = Sheet::text(row.reply).size(page);
            let measure = grid.measure;
            let destination = match &passage.kind {
                PassageKind::Choice {
                    destination: Some(target),
                } if target != "END" => Some(format!("→ {}", display_name(target))),
                PassageKind::Dialogue { .. } => joined
                    .filter(|target| *target != "END")
                    .map(|target| format!("→ {}", display_name(target))),
                PassageKind::Choice { .. } => None,
            };
            let wide = grid.margin > 0.;
            let route = |target: String| {
                Sheet::Destination
                    .label(page, target)
                    .color(state.palette.teal_text)
                    .prop(LineBreaking::WordWrap)
            };
            let column = grid.destination > 0.;
            flex_col((
                flex_row((
                    wide.then(|| margin(state, grid, label, number)),
                    // Fields keep 2px of padding and border before their text; so does this.
                    sized_box(sheet_text(passage.text.clone(), size, state.palette.ink))
                        .padding(Padding::all(size::FIELD_INSET.px()))
                        .width(measure.px()),
                    column.then(|| destination.clone()).flatten().map(|target| {
                        sized_box(route(target).width(grid.destination.px())).padding(
                            Padding::left(
                                (DESTINATION_GUTTER - GUTTER)
                                    .max(control_inset(state.ui_scale()))
                                    .px(),
                            ),
                        )
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
                .gap(grid.gutter().px()),
                (!column).then_some(destination).flatten().map(|target| {
                    // Under the text, where a route button's words would start.
                    sized_box(route(target).width(grid.measure.px())).padding(Padding::left(
                        (grid.text() + control_inset(state.ui_scale())).px(),
                    ))
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
                Sheet::Dialogue.size(page),
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
        Content::Continuation(target) => continuation_row(
            state,
            grid,
            None::<Label>,
            // As tall as the route button it stands for, so the row keeps the editable height.
            flex_col((Sheet::Destination
                .label(page, format!("→ {}", display_name(target)))
                .color(state.palette.teal_text)
                .prop(LineBreaking::WordWrap)
                .width(
                    if grid.destination > 0. {
                        grid.destination
                    } else {
                        grid.measure
                    }
                    .px(),
                ),))
            .main_axis_alignment(MainAxisAlignment::Center)
            .dims(Dimensions::AUTO.with_height((size::ROW * f64::from(state.ui_scale())).px())),
            RouteFace::Label,
        )
        .boxed(),
    }
}
