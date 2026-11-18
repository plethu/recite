//! Runtime controls; capture policy and traversal stay in the shared owners.
use super::*;
use crate::app::keyboard::SlipHost;
use crate::presentation::grid::Grid;
use crate::presentation::{Page, pane, space};
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{group::named, print},
    trial::Review,
};
use recite_writer_app::trial::captions;
use recite_writer_model::PreviewPage;
use recite_writer_session::preview::catalogue_stale;
use xilem::masonry::layout::UnitPoint;
use xilem::masonry::properties::LineBreaking;
use xilem::style::Padding;
use xilem::view::{GridExt, GridTrackSize, grid as grid_view, zstack};
mod context;
mod controls;
mod feedback;
mod inputs;
mod playback;
mod rows;
mod trace;

/// Preview is a sheet on the desk, like Script: its head stays put with the run's actions while the
/// transcript scrolls beneath it on the same grid as the galley.
pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let grid = super::workspace::sheet::page_grid(state);
    named(
        "Conversation preview",
        super::workspace::sheet::paper(
            state,
            (&grid, state.content_width()),
            flex_col((
                controls::header(state, &grid),
                zstack((
                    pane("Transcript", columns(&grid, content(state, &grid)))
                        .constrain_horizontal(true),
                    state
                        .configuring_trial()
                        .then(|| inputs::slip(state, &grid)),
                ))
                .alignment(UnitPoint::TOP_LEFT)
                .prop(SlipHost(true))
                .flex(1.),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px())
            .prop(Region(Some(Pane::Preview))),
        ),
    )
}

/// The transcript's columns, padded to the grid under the head.
fn columns<V: WidgetView<Writer>>(grid: &Grid, content: V) -> impl WidgetView<Writer> + use<V> {
    flex_col((content,))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .padding(Padding {
            top: space::TEXT.px(),
            bottom: space::REGIONS.px(),
            left: grid.pad.px(),
            right: grid.pad.px(),
        })
}

/// `child` set in the text column, where the galley's lines start.
fn prose_column<V: WidgetView<Writer>>(grid: &Grid, child: V) -> impl WidgetView<Writer> + use<V> {
    sized_box(child).padding(Padding::left(grid.text().px()))
}

fn content(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing || state.session.read_only();
    let busy = state.trial.busy();
    let model = state.session.model();
    let page = model.preview_page().filter(|_| state.trial.visible);
    let setup_first = state.configuring_trial() && page.is_none_or(|page| page.ended);
    let measure = grid.measure;
    flex_col((
        // Before a run the status is the head's caption instead, so the head keeps its height.
        (state.session.model().preview_provenance().is_some())
            .then(|| status(state))
            .flatten()
            .map(|status| {
                prose_column(
                    grid,
                    print::meta(scale, status.to_owned())
                        .color(state.palette.muted)
                        .prop(LineBreaking::WordWrap)
                        .width(measure.px()),
                )
            }),
        matches!(state.trial.settings.review, Review::Diagnostic)
            .then(|| prose_column(grid, feedback::details(state, measure))),
        grid_view((
            flex_col((page.map(|page| running(state, grid, page, closing || busy)),))
                .grid_pos(0, u16::from(setup_first)),
            flex_col((setup(state, grid),)).grid_pos(0, u16::from(!setup_first)),
        ))
        .columns([GridTrackSize::FRACTION])
        .rows([GridTrackSize::Auto; 2])
        .gap(space::TEXT.px()),
        state
            .trial
            .visible
            .then(|| trace::panel(state, closing))
            .flatten(),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .prop(Region(Some(Pane::Preview)))
}

/// The run's status sentence while there is one to show: none beside feedback or a diagnostic.
fn status(state: &Writer) -> Option<&str> {
    (state.trial.visible
        && state.trial_feedback().is_none()
        && !matches!(state.trial.settings.review, Review::Diagnostic))
    .then(|| state.trial_status())
    .flatten()
}

fn setup(state: &Writer, grid: &Grid) -> Option<impl WidgetView<Writer> + use<>> {
    state.reading.script().is_err().then(|| {
        prose_column(
            grid,
            paragraph(
                state.ui_scale(),
                "Repair this source in your editor before starting a preview.",
                grid.measure,
            ),
        )
    })
}

fn running(
    state: &Writer,
    grid: &Grid,
    page: &PreviewPage,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let stale = state.session.model().preview_stale()
        || state
            .session
            .model()
            .preview_provenance()
            .is_some_and(|snapshot| {
                catalogue_stale(snapshot, state.localisation.preview_catalogue())
            });
    flex_col((
        stale.then(|| {
            prose_column(
                grid,
                paragraph(
                    state.ui_scale(),
                    "This preview uses older inputs. Restart to use your accepted edits.",
                    grid.measure,
                ),
            )
        }),
        playback::playback(state, grid, page, disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .prop(PaneEntry(
        !state.configuring_trial() && state.unfinished_trial_edit().is_none(),
    ))
}
