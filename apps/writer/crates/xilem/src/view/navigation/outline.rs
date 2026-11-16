//! The open scene's beats, nested under its row in the scene list: one row each, the current
//! one selected, a problem repeated as the margin wedge.
use super::*;
use crate::app::keyboard::{RenamesBeat, ReturnFocus};
use crate::presentation::double_click::double_click;
use crate::presentation::marks::{GLYPH, wedge_glyph};
use crate::presentation::slip::OFFSET;
use crate::workspaces::rename::inline::{self as rename, Actions, Place, Target};
use recite_compiler::authoring::BlockDefinitionSummary;

/// What every beat row of the page shares.
struct Rows<'a> {
    selected: Option<&'a str>,
    problems: Vec<(String, String)>,
    /// The width of a row's words.
    width: f64,
    closing: bool,
}

pub(super) fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.project_action_blocked();
    let blocks = state.session.model().document().block_summaries();
    // Short scenes use the slots otherwise occupied by pagination controls.
    let page_size = beat_page_size(blocks.len());
    let pages = blocks.len().div_ceil(page_size).max(1);
    let page = state.beat_page.min(pages - 1);
    let rows = Rows {
        selected: if state.visible_workspace() == Workspace::Map {
            state.map_selected()
        } else {
            state.reading.selected.as_deref()
        },
        problems: state.beat_problems(),
        width: sidebar_width(state) - sidebar::ROW - sidebar::OUTLINE,
        closing,
    };
    let shown = blocks
        .iter()
        .skip(page * page_size)
        .take(page_size)
        .collect::<Vec<_>>();
    // The row renamed in place splits the page, so the other rows stay plain rows.
    let renamed = state
        .inline_rename()
        .filter(|inline| inline.place == Place::Outline)
        .and_then(|inline| {
            shown
                .iter()
                .position(|block| block.id().as_str() == inline.beat)
        });
    let split = renamed.unwrap_or(shown.len());
    let before = shown[..split]
        .iter()
        .map(|block| beat_row(state, &rows, block))
        .collect::<Vec<_>>();
    let renamed = renamed.map(|index| renamed_row(state, &rows, shown[index]));
    let after = shown[(split + 1).min(shown.len())..]
        .iter()
        .map(|block| beat_row(state, &rows, block))
        .collect::<Vec<_>>();
    flex_col((
        flex_col((before, renamed, after))
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

/// A beat's row: pressing it reads the beat; F2 or a double-click renames it in place.
fn beat_row(
    state: &Writer,
    rows: &Rows<'_>,
    block: &BlockDefinitionSummary,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let id = block.id().to_string();
    let current = rows.selected == Some(id.as_str());
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
    let problem = rows.problems.iter().any(|(beat, _)| beat == &id);
    let target = Target {
        beat: id.clone(),
        place: Place::Outline,
    };
    let returning = state.rename_return() == Some(&target);
    let double = target.clone();
    let words = one_line(text.clone(), Type::Control, scale, state.palette.ink);
    // The current beat is the list's selected row: the plate, the ink bar and the state.
    // A beat with a problem repeats the margin wedge, and its name says so.
    double_click(
        listed(
            &state.palette,
            current,
            row(
                current,
                // One flex either way, so a problem's wedge mounts no wrapper of its own.
                flex_row((
                    words.flex(1.),
                    problem.then(|| wedge_glyph(GLYPH, state.palette.error)),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::UNIT.px())
                .width(rows.width.px()),
                move |state: &mut Writer| state.inspect(id.clone()),
            )
            .accessible_label(if problem {
                format!("{text} · Problem")
            } else {
                text
            })
            .disabled(rows.closing)
            .prop(RenamesBeat(Some(target)))
            .prop(ReturnFocus(returning)),
        ),
        move |state: &mut Writer| state.rename_beat_in(double.clone()),
    )
}

/// The row renamed in place: its name as the field until the review is ready, then the row
/// again, with the review's progress or consequence slip under it.
fn renamed_row(
    state: &Writer,
    rows: &Rows<'_>,
    block: &BlockDefinitionSummary,
) -> impl WidgetView<Writer> + use<> {
    let id = block.id().as_str();
    let name = match rename::editing(state, id, Place::Outline) {
        Some(inline) => Either::A(rename::field(inline, Type::Control, state.ui_scale())),
        None => Either::B(beat_row(state, rows, block)),
    };
    // As wide as the row less the slip's stock, so its offset shows beside the scroll bar; the
    // row keeps the slip at that width while the field stretches across the row.
    let row = rows.width + sidebar::ROW - sidebar::COLUMN;
    let slip = rename::consequence(state, Place::Outline, row - OFFSET, Actions::Stacked)
        .map(|slip| flex_row((slip,)));
    flex_col((name, slip))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(space::UNIT.px())
}
