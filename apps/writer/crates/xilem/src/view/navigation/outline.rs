//! The open scene's beats, nested under its row in the scene list: one row each, the current
//! one selected, a problem repeated as the margin wedge.
use super::*;
use crate::app::keyboard::{RenamesBeat, ReturnFocus, SlipAnchor};
use crate::presentation::double_click::double_click;
use crate::presentation::marks::{GLYPH, wedge_glyph};
use crate::presentation::print;
use crate::workspaces::rename::inline::{self as rename, Place, Target};
use recite_compiler::authoring::BlockDefinitionSummary;

/// What every beat row shares.
struct Rows<'a> {
    selected: Option<&'a str>,
    problems: Vec<(String, String)>,
    /// The width of a row's words.
    width: f64,
    closing: bool,
}

/// The most beats a scene lists in full.
const ALL_BEATS: usize = 12;
/// How many beats a longer scene lists around the selected one.
const WINDOW: usize = 8;

pub(super) fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.project_action_blocked();
    let blocks = state.session.model().document().block_summaries();
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
    // A scene of up to ALL_BEATS beats lists them all; a longer one lists the WINDOW around the
    // selected beat, which moves with the selection, and counts the rest. Nothing pages.
    let selected = blocks
        .iter()
        .position(|block| Some(block.id().as_str()) == rows.selected)
        .unwrap_or(0);
    let (start, end) = if blocks.len() <= ALL_BEATS {
        (0, blocks.len())
    } else {
        let start = selected
            .saturating_sub(WINDOW / 2)
            .min(blocks.len() - WINDOW);
        (start, start + WINDOW)
    };
    let shown = blocks[start..end].iter().collect::<Vec<_>>();
    let more = |count: usize, place: &str| {
        (count > 0).then(|| {
            print::meta(
                scale,
                format!("{count} more beats {place} · Find reaches them"),
            )
            .color(state.palette.muted)
        })
    };
    let (above, below) = (more(start, "above"), more(blocks.len() - end, "below"));
    // The row renamed in place splits the list, so the other rows stay plain rows.
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
    flex_col((above, before, renamed, after, below))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(0.px())
        .padding(xilem::style::Padding::left(sidebar::OUTLINE.px()))
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
/// again, which the review's progress or consequence slip hangs from.
fn renamed_row(
    state: &Writer,
    rows: &Rows<'_>,
    block: &BlockDefinitionSummary,
) -> impl WidgetView<Writer> + use<> {
    let id = block.id().as_str();
    let name = match rename::editing(state, id, Place::Outline) {
        Some(inline) => Either::A(rename::field(
            state,
            inline,
            Type::Control,
            state.ui_scale(),
        )),
        None => Either::B(beat_row(state, rows, block)),
    };
    // The review's slip floats over the window from under the row, so no row moves.
    sized_box(name).prop(SlipAnchor(true))
}
