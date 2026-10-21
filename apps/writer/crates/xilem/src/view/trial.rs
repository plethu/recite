//! Runtime controls; capture policy and traversal stay in the shared owners.
use super::*;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{group::named, manuscript_text, print},
};
use recite_writer_model::PreviewPage;
use recite_writer_session::preview::catalogue_stale;
use xilem::view::{GridExt, GridTrackSize, grid, portal};
mod captions;
mod inputs;
mod playback;
mod trace;

pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    named(
        "Conversation preview",
        flex_col((
            header(state),
            portal(content(state)).constrain_horizontal(true).flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(8.px())
        .prop(Region(Some(Pane::Preview))),
    )
}
fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let wide = state.viewport.width - state.navigation_width() >= 600. * f64::from(scale);
    grid((
        label(scale, "Conversation preview")
            .text_size(18. * scale)
            .grid_pos(0, 0),
        flex_row((
            print::action(
                scale,
                "Return to writing",
                state.phase != Phase::Writing,
                Writer::return_from_trial,
            ),
            print::action(
                scale,
                if state.configuring_trial() {
                    "Hide preview inputs"
                } else {
                    "Configure preview"
                },
                state.phase != Phase::Writing,
                |state: &mut Writer| {
                    if state.configuring_trial() {
                        state.close_trial_inputs();
                    } else {
                        state.configure_trial();
                    }
                },
            ),
        ))
        .gap(8.px())
        .grid_pos(u16::from(wide), u16::from(!wide)),
    ))
    .columns([
        GridTrackSize::FRACTION,
        if wide {
            GridTrackSize::Auto
        } else {
            GridTrackSize::Fixed(0.px())
        },
    ])
    .rows([GridTrackSize::Auto; 2])
    .gap(8.px())
}
fn content(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing || state.session.read_only();
    let busy = state.trial.busy();
    let model = state.session.model();
    let page = model.preview_page().filter(|_| state.trial.visible);
    flex_col((
        state
            .trial
            .visible
            .then(|| paragraph(scale, state.trial.status.clone(), state.script_width())),
        grid((
            flex_col((page.map(|page| running(state, page, closing || busy)),))
                .grid_pos(0, u16::from(state.configuring_trial())),
            setup(state, closing, busy).grid_pos(0, u16::from(!state.configuring_trial())),
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

fn setup(state: &Writer, closing: bool, busy: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_col((
        flex_row((
            text_button(scale, "Start / restart preview", Writer::start_trial).disabled(
                closing
                    || busy
                    || state.localisation.preview_blocked()
                    || state.reading.script().is_err()
                    || state.reading.script().is_ok_and(<[_]>::is_empty),
            ),
            text_button(scale, "Cancel preparation", |state: &mut Writer| {
                state.trial.cancel()
            })
            .disabled(closing || !busy),
        ))
        .gap(8.px()),
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
    .prop(PaneEntry(state.configuring_trial()))
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
            playback::playback(state, page, disabled),
            state
                .session
                .model()
                .preview_provenance()
                .map(|snapshot| paragraph(scale, snapshot.caption.clone(), state.script_width())),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px())
        .prop(PaneEntry(!state.configuring_trial())),
    )
    .padding(12.px())
    .background_color(state.palette.inset)
}
