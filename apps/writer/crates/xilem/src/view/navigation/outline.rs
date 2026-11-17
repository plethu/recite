//! The open scene's beats, nested under its row in the scene list: one row each, the current
//! one selected, a problem repeated as the margin wedge. A long scene's beats scroll in a list
//! of their own, which keeps the selected beat in view.
use super::*;
use crate::app::keyboard::{RenamesBeat, ReturnFocus, SlipAnchor};
use crate::presentation::double_click::double_click;
use crate::presentation::marks::{GLYPH, wedge_glyph};
use crate::presentation::tokens::size;
use crate::workspaces::rename::inline::{self as rename, Place, Target};
use recite_compiler::authoring::BlockDefinitionSummary;
use xilem::core::MessageResult;
use xilem::masonry::layout::UnitPoint;
use xilem::style::Padding;
use xilem::view::{virtual_scroll, zstack};

/// What every beat row shares.
struct Rows<'a> {
    selected: Option<&'a str>,
    problems: Vec<(String, String)>,
    /// The width of a row's words.
    width: f64,
    closing: bool,
}

/// The most beats a scene lists in place; a longer scene's beats scroll in a list of their own.
const ALL_BEATS: usize = 12;
/// How many rows a scrolling beat list shows: a half row past the last whole one shows there are
/// more, beside its scroll thumb.
const LIST_ROWS: f64 = 6.5;
/// How far past its view a scrolling beat list keeps rows built, in view lengths: one row each
/// way, so Tab and Shift+Tab reach the next beat, which then scrolls into view.
const OVERSCAN: f64 = 1. / LIST_ROWS;

/// What every beat row shares, read afresh for each row a scrolling list builds.
fn shared_rows(state: &Writer) -> Rows<'_> {
    Rows {
        selected: if state.visible_workspace() == Workspace::Map {
            state.map_selected()
        } else {
            state.reading.selected.as_deref()
        },
        problems: state.beat_problems(),
        width: sidebar_width(state) - sidebar::ROW - sidebar::OUTLINE,
        closing: state.project_action_blocked(),
    }
}

pub(super) fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocks = state.session.model().document().block_summaries();
    let rows = shared_rows(state);
    let count = blocks.len();
    let inset = xilem::style::Padding::left(sidebar::OUTLINE.px());
    if count <= ALL_BEATS {
        // The row renamed in place splits the list, so the other rows stay plain rows.
        let renamed = state
            .inline_rename()
            .filter(|inline| inline.place == Place::Outline)
            .and_then(|inline| {
                blocks
                    .iter()
                    .position(|block| block.id().as_str() == inline.beat)
            });
        let split = renamed.unwrap_or(count);
        let plain = |blocks: &[BlockDefinitionSummary]| {
            blocks
                .iter()
                .map(|block| beat_row(state, &rows, &Beat::of(block)))
                .collect::<Vec<_>>()
        };
        let before = plain(&blocks[..split]);
        let renamed = renamed.map(|index| renamed_row(state, &rows, &Beat::of(&blocks[index])));
        let after = plain(&blocks[(split + 1).min(count)..]);
        return Either::A(
            flex_col((before, renamed, after))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
                .padding(inset),
        );
    }
    // A longer scene scrolls its beats in a virtual list that builds only the rows in view and
    // one either side, and follows the selected beat, so a scene of thousands of beats mounts
    // about ten rows.
    let selected = blocks
        .iter()
        .position(|block| Some(block.id().as_str()) == rows.selected);
    let height = LIST_ROWS * size::ROW * f64::from(state.ui_scale());
    Either::B(
        zstack((
            virtual_scroll(count, |state: &mut Writer, position| {
                let state = &*state;
                listed_row(state, &shared_rows(state), position)
            })
            // Each beat is a Tab stop; the list itself is not.
            .accessible_label(BEATS)
            .focusable(false)
            .overscan(OVERSCAN, OVERSCAN)
            .reveal(selected)
            .on_scroll(|state: &mut Writer, range| {
                state.attention.show_beats(range);
                MessageResult::Action(())
            })
            .dims(((sidebar_width(state) - sidebar::COLUMN).px(), height.px())),
            thumb(state, count, selected, height),
        ))
        .alignment(UnitPoint::TOP_RIGHT)
        // A gap under the scene's row, so no beat is cut off flush against it.
        .padding(Padding {
            top: space::TEXT.px(),
            ..inset
        }),
    )
}

/// The scrolling beat list's thumb, drawn as every other scroll region's: where the beats in
/// view sit among all of them. Pointer and keys scroll the list itself.
fn thumb(
    state: &Writer,
    count: usize,
    selected: Option<usize>,
    height: f64,
) -> impl WidgetView<Writer> + use<> {
    let range = state.attention.beats_in_view().unwrap_or_else(|| {
        let start = selected.unwrap_or(0);
        start..start + LIST_ROWS.ceil() as usize
    });
    let (length, top) = thumb_place(count, range, height);
    sized_box(
        sized_box(flex_col(()))
            .dims((THUMB.px(), length.px()))
            .background_color(state.palette.boundary)
            .corner_radius((THUMB / 2.).px()),
    )
    .padding(Padding {
        top: top.px(),
        right: THUMB_PAD.px(),
        ..Padding::all(0.px())
    })
}

/// A scroll thumb's width and its inset from the list's edge, as Masonry draws its own.
const THUMB: f64 = 8.;
const THUMB_PAD: f64 = 2.;
/// The shortest a thumb is drawn, as Masonry draws its own.
const THUMB_MIN: f64 = 45.;

/// The thumb's length and offset in a track `height` long, for `range` of `count` rows in view.
pub(super) fn thumb_place(count: usize, range: std::ops::Range<usize>, height: f64) -> (f64, f64) {
    let count = count.max(1) as f64;
    let track = height - 2. * THUMB_PAD;
    let shown = (LIST_ROWS / count).min(1.);
    let length = (track * shown).max(THUMB_MIN.min(track));
    let scrolled = range.start as f64 / (count - range.len() as f64).max(1.);
    let top = THUMB_PAD + (track - length) * scrolled.clamp(0., 1.);
    (length, top)
}
/// The scrolling beat list's name.
pub(super) const BEATS: &str = "Beats";

/// What a beat row shows of its beat.
#[derive(Default)]
struct Beat {
    id: String,
    start: bool,
    end: bool,
}
impl Beat {
    fn of(block: &BlockDefinitionSummary) -> Self {
        Self {
            id: block.id().to_string(),
            start: block.is_default(),
            end: block.has_end_destination(),
        }
    }
}

/// The scrolling list's row at `position`: renamed in place or plain. A list can ask once for a
/// row the beats no longer have, before it learns their new count; that row is blank and
/// disabled, so every row keeps one shape and mounts no wrapper.
fn listed_row(state: &Writer, rows: &Rows<'_>, position: usize) -> impl WidgetView<Writer> + use<> {
    let beat = state
        .session
        .model()
        .document()
        .block_summaries()
        .get(position)
        .map(Beat::of)
        .unwrap_or_default();
    let renamed = state
        .inline_rename()
        .is_some_and(|inline| inline.place == Place::Outline && inline.beat == beat.id);
    if renamed {
        Either::A(renamed_row(state, rows, &beat))
    } else {
        Either::B(beat_row(state, rows, &beat))
    }
}

/// A beat's row: pressing it reads the beat; F2 or a double-click renames it in place.
fn beat_row(state: &Writer, rows: &Rows<'_>, beat: &Beat) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let id = beat.id.clone();
    let blank = id.is_empty();
    let current = rows.selected == Some(id.as_str());
    let text = format!(
        "{}{}{}",
        if beat.start { "Start · " } else { "" },
        display_name(&id),
        if beat.end { " · end" } else { "" }
    );
    let problem = rows.problems.iter().any(|(beat, _)| beat == &id);
    let target = Target {
        beat: id.clone(),
        place: Place::Outline,
    };
    let returning = state.rename_return() == Some(&target);
    let double = target.clone();
    // A problem is said in a word beside its wedge, and in the name.
    let words = one_line(
        if problem {
            format!("{text} · error")
        } else {
            text.clone()
        },
        Type::Control,
        scale,
        state.palette.ink,
    );
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
            .disabled(rows.closing || blank)
            .prop(RenamesBeat(Some(target)))
            .prop(ReturnFocus(returning)),
        ),
        move |state: &mut Writer| state.rename_beat_in(double.clone()),
    )
}

/// The row renamed in place: its name as the field until the review is ready, then the row
/// again, which the review's progress or consequence slip hangs from.
fn renamed_row(state: &Writer, rows: &Rows<'_>, beat: &Beat) -> impl WidgetView<Writer> + use<> {
    let name = match rename::editing(state, &beat.id, Place::Outline) {
        Some(inline) => Either::A(rename::field(
            state,
            inline,
            Type::Control,
            state.ui_scale(),
        )),
        None => Either::B(beat_row(state, rows, beat)),
    };
    // The review's slip floats over the window from under the row, so no row moves.
    sized_box(name).prop(SlipAnchor(true))
}
