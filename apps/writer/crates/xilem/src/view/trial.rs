//! Runtime controls; capture policy and traversal stay in the shared owners.
use super::*;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{group::named, manuscript_text, print},
    trial::Review,
};
use recite_writer_model::PreviewPage;
use recite_writer_session::preview::catalogue_stale;
use xilem::view::{GridExt, GridTrackSize, grid, portal};
mod captions;
mod controls;
mod feedback;
mod inputs;
mod playback;
mod trace;

pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    named(
        "Conversation preview",
        flex_col((
            controls::header(state),
            portal(content(state)).constrain_horizontal(true).flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(8.px())
        .prop(Region(Some(Pane::Preview))),
    )
}
fn content(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing || state.session.read_only();
    let busy = state.trial.busy();
    let model = state.session.model();
    let page = model.preview_page().filter(|_| state.trial.visible);
    flex_col((
        (state.trial.visible && !matches!(state.trial.settings.review, Review::Diagnostic))
            .then(|| paragraph(scale, state.trial.status.clone(), state.script_width())),
        matches!(state.trial.settings.review, Review::Diagnostic).then(|| feedback::details(state)),
        grid((
            flex_col((page.map(|page| running(state, page, closing || busy)),))
                .grid_pos(0, u16::from(state.configuring_trial())),
            setup(state, closing).grid_pos(0, u16::from(!state.configuring_trial())),
        ))
        .columns([GridTrackSize::FRACTION])
        .rows([GridTrackSize::Auto; 2])
        .gap(12.px()),
        state.trial.visible.then(|| trace::panel(state, closing)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Preview)))
}

fn setup(state: &Writer, closing: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_col((
        inputs::controls(state, closing),
        state.reading.script().is_err().then(|| {
            paragraph(
                scale,
                "Repair this source in your editor before starting a preview.",
                state.script_width(),
            )
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn running(state: &Writer, page: &PreviewPage, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let stale = state.session.model().preview_stale()
        || state
            .session
            .model()
            .preview_provenance()
            .is_some_and(|snapshot| {
                catalogue_stale(snapshot, state.localisation.preview_catalogue())
            });
    sized_box(
        flex_col((
            stale.then(|| {
                paragraph(
                    scale,
                    "This preview uses older inputs. Restart to use your accepted edits.",
                    state.script_width(),
                )
            }),
            state
                .session
                .model()
                .preview_provenance()
                .map(|snapshot| paragraph(scale, snapshot.caption.clone(), state.script_width())),
            playback::playback(state, page, disabled),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px())
        .prop(PaneEntry(!state.configuring_trial())),
    )
    .padding(12.px())
    .background_color(state.palette.inset)
}
