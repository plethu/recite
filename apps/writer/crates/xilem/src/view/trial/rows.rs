//! The galley's rows in a preview: shown lines with their speakers, the transcript of past turns,
//! and the numbered replies, on the same grid and in the same roles as Script.
use super::*;
use crate::app::keyboard::ReturnFocus;
use crate::presentation::Sheet;
use crate::presentation::control_inset;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::row::named_row;
use crate::presentation::sheet_text::sheet_text;

/// A numbered reply as the galley sets one: the number in the margin column, "REPLIES" beside
/// the first, and the text on the measure. The whole row is the reply's button, its number its
/// digit key; the margin narrows by the button's inset so the text keeps the galley's column.
pub(super) fn reply(
    state: &Writer,
    grid: &Grid,
    index: usize,
    text: String,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let page = Page::of(state);
    let inset = control_inset(state.ui_scale());
    let number = index + 1;
    let margin = (grid.margin > 0.).then(|| {
        let narrowed = Grid {
            margin: (grid.margin - inset).max(0.),
            ..*grid
        };
        let caption = if index == 0 { "REPLIES" } else { "" };
        super::super::reading::margin(state, &narrowed, caption.to_owned(), Some(number))
    });
    let inline = (grid.margin <= 0.).then(|| {
        Sheet::Number
            .label(page, number.to_string())
            .color(state.palette.muted)
    });
    styled(
        ControlStyle::Action,
        named_row(
            text.clone(),
            flex_row((
                margin,
                inline,
                sheet_text(text, Sheet::Reply.size(page), state.palette.ink)
                    .width(grid.measure.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
            .gap(
                if grid.margin > 0. {
                    grid.gutter()
                } else {
                    space::TEXT
                }
                .px(),
            ),
            move |state: &mut Writer| state.choose_preview(index),
        )
        .disabled(disabled)
        // The first reply takes focus when a step's Continue or chosen reply goes away.
        .prop(ReturnFocus(index == 0)),
    )
}

/// A shown line on the galley's grid: its speaker in the margin column in the Label role, or
/// above the text where the paper has no margin column.
pub(super) fn line<V: WidgetView<Writer>>(
    state: &Writer,
    grid: &Grid,
    speaker: Option<&str>,
    text: V,
) -> impl WidgetView<Writer> + use<V> {
    let speaker = speaker.unwrap_or("").to_uppercase();
    let wide = grid.margin > 0.;
    let above = (!wide && !speaker.is_empty()).then(|| {
        Sheet::Label
            .label(Page::of(state), speaker.clone())
            .color(state.palette.accent)
    });
    flex_col((
        above,
        flex_row((
            wide.then(|| super::super::reading::margin(state, grid, speaker, None)),
            text,
        ))
        .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
        .gap(grid.gutter().px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
}

/// What the preview has already shown, oldest first and muted: each page's lines with their
/// speakers, and the reply chosen after them in the reply tier, with a hairline closing each turn
/// before the next.
pub(super) fn transcript(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    let measure = grid.measure;
    let page = Page::of(state);
    let muted = state.palette.muted;
    let turns = state
        .session
        .model()
        .preview_transcript()
        .iter()
        .map(|turn| {
            flex_col((
                (!turn.text.is_empty()).then(|| {
                    line(
                        state,
                        grid,
                        turn.speaker.as_deref(),
                        sheet_text(turn.text.clone(), Sheet::Dialogue.size(page), muted)
                            .width(measure.px()),
                    )
                }),
                turn.reply.as_ref().map(|reply| {
                    line(
                        state,
                        grid,
                        Some("Reply"),
                        sheet_text(reply.clone(), Sheet::Reply.size(page), muted)
                            .width(measure.px()),
                    )
                }),
                prose_column(
                    grid,
                    sized_box(print::hairline(&state.palette)).width(measure.px()),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        })
        .collect::<Vec<_>>();
    flex_col(turns)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
}
