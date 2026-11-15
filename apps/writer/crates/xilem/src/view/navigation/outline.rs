//! The open scene's beats, nested under its row in the scene list: one row each, the current
//! one selected, a problem repeated as the margin wedge.
use super::*;
use crate::presentation::marks::{GLYPH, wedge_glyph};

pub(super) fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.project_action_blocked();
    let blocks = state.session.model().document().block_summaries();
    // Short scenes use the slots otherwise occupied by pagination controls.
    let page_size = beat_page_size(blocks.len());
    let pages = blocks.len().div_ceil(page_size).max(1);
    let page = state.beat_page.min(pages - 1);
    let selected = if state.visible_workspace() == Workspace::Map {
        state.map_selected()
    } else {
        state.reading.selected.as_deref()
    };
    let problems = state.beat_problems();
    let width = sidebar_width(state) - sidebar::ROW - sidebar::OUTLINE;
    let entries = blocks
        .iter()
        .skip(page * page_size)
        .take(page_size)
        .map(|block| {
            let id = block.id().to_string();
            let current = selected == Some(id.as_str());
            let text = format!(
                "{}{}{}",
                if block.is_default() { "Start · " } else { "" },
                display_name(&id),
                if block.has_end_destination() {
                    " · end"
                } else {
                    ""
                }
            );
            let problem = problems.iter().any(|(beat, _)| beat == &id);
            // The current beat is the list's selected row: the plate, the ink bar and the state.
            // A beat with a problem repeats the margin wedge, and its name says so.
            listed(
                &state.palette,
                current,
                row(
                    current,
                    if problem {
                        Either::A(
                            flex_row((
                                one_line(text.clone(), Type::Control, scale, state.palette.ink)
                                    .flex(1.),
                                wedge_glyph(GLYPH, state.palette.error),
                            ))
                            .cross_axis_alignment(CrossAxisAlignment::Center)
                            .gap(space::UNIT.px())
                            .width(width.px()),
                        )
                    } else {
                        Either::B(
                            one_line(text.clone(), Type::Control, scale, state.palette.ink)
                                .width(width.px()),
                        )
                    },
                    move |state: &mut Writer| state.inspect(id.clone()),
                )
                .named(problem.then(|| format!("{text} · Problem").into()))
                .disabled(closing),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(entries)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px())
            .padding(xilem::style::Padding::left(sidebar::OUTLINE.px())),
        (pages > 1).then(|| {
            flex_col((
                text_button(scale, "Previous beats", move |state: &mut Writer| {
                    state.beat_page = page.saturating_sub(1)
                })
                .disabled(closing || page == 0),
                text_button(scale, "Next beats", move |state: &mut Writer| {
                    state.beat_page = (page + 1).min(pages - 1)
                })
                .disabled(closing || page + 1 >= pages),
            ))
            .gap(space::SIBLINGS.px())
        }),
        (pages > 1).then(|| label(scale, format!("Beats · page {} of {pages}", page + 1))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
