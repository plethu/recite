//! A short captured summary stays subordinate to prose; full provenance is explicit.
use super::*;

/// The head's toggle for the run's full provenance, once a run has captured one.
pub(super) fn toggle(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    state.session.model().preview_provenance()?;
    Some(print::action_disclosing(
        state.ui_scale(),
        // The sheet's own head says preview; the name keeps it for assistive technology.
        ("Context", "Preview context"),
        (state.trial.context_details, false),
        |state: &mut Writer| state.trial.context_details = !state.trial.context_details,
    ))
}

/// The run's full provenance under the head's caption, while shown.
pub(super) fn details(state: &Writer, width: f64) -> Option<impl WidgetView<Writer> + use<>> {
    let snapshot = state.session.model().preview_provenance()?;
    state.trial.context_details.then(|| {
        flex_row((print::meta(state.ui_scale(), snapshot.caption.clone())
            .prop(LineBreaking::WordWrap)
            .color(state.palette.muted)
            .width(width.px()),))
    })
}
