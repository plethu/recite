//! Runtime controls; capture policy and traversal stay in the shared owners.
use super::*;
use crate::{
    attention::{Pane, Region},
    presentation::manuscript_text,
};
use recite_writer_model::PreviewPage;
use recite_writer_session::preview::catalogue_stale;
mod captions;
mod inputs;
mod playback;
mod trace;

pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing || state.session.read_only();
    let busy = state.trial.busy();
    let model = state.session.model();
    let page = model.preview_page().filter(|_| state.trial.visible);
    flex_col((
        label(scale, "Conversation preview").text_size(22. * scale),
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
            text_button(scale, "Hide preview", |state: &mut Writer| {
                state.trial.cancel();
                state.trial.visible = false;
            })
            .disabled(closing || !state.trial.visible),
        ))
        .gap(8.px()),
        inputs::controls(state, closing),
        (state.reading.script().is_err() && !state.trial.visible).then(|| {
            paragraph(
                scale,
                "Repair this source in your editor before starting a preview.",
                state.script_width(),
            )
        }),
        state.trial.visible.then(|| {
            flex_col((
                paragraph(scale, state.trial.status.clone(), state.script_width()),
                page.map(|page| running(state, page, closing || busy)),
                trace::panel(state, closing),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Preview)))
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
            label(scale, "Running preview").text_size(20. * scale),
            state
                .session
                .model()
                .preview_provenance()
                .map(|snapshot| paragraph(scale, snapshot.caption.clone(), state.script_width())),
            stale.then(|| {
                paragraph(
                    scale,
                    "This preview uses older inputs. Restart to use your accepted edits.",
                    state.script_width(),
                )
            }),
            playback::playback(state, page, disabled),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .padding(12.px())
    .background_color(state.palette.inset)
}
