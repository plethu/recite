//! The preview sheet's head: the title in Display with the run's captured context as its caption,
//! the run's actions, and the close icon back to writing. It stays put above the transcript.
use super::*;
use crate::presentation::sheet_head::{Close, Layout, sheet_head};
use xilem::masonry::core::ArcStr;

/// The head width at 100% below which the run's actions, up to six, leave the title's row.
const HEAD_ROW: f64 = 900.;

pub(super) fn header(state: &Writer, grid: &Grid) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = grid.width - 2. * grid.pad - grid.text();
    let caption = state
        .session
        .model()
        .preview_provenance()
        .map(|snapshot| ArcStr::from(snapshot.summary.clone()));
    sized_box(
        flex_col((
            sheet_head(
                Layout {
                    scale,
                    // Narrower than this, the run's actions take a row under the title.
                    stacked: width < HEAD_ROW * f64::from(scale),
                },
                &state.palette,
                ("Preview", caption, (actions(state),)),
                Close {
                    name: "Return to writing",
                    disabled: state.phase() != Phase::Writing,
                    action: Writer::return_from_trial,
                },
            ),
            context::details(state, width),
            feedback::panel(state, width),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(space::SIBLINGS.px()),
    )
    .padding(Padding {
        top: if grid.sheet {
            space::REGIONS
        } else {
            space::KINDS
        }
        .px(),
        bottom: space::SIBLINGS.px(),
        left: (grid.pad + grid.text()).px(),
        right: grid.pad.px(),
    })
}

/// Whether the run waits only for the writer to go on: no replies, question or effect pending.
fn can_continue(page: &PreviewPage) -> bool {
    page.choices.is_empty()
        && !page.ended
        && page.condition.is_none()
        && page.waiting_effect.is_none()
}

/// The run's actions. Continue is the one Primary while the run can go on; otherwise starting
/// or restarting is, unless the run waits on a reply or answer, when no action outranks those.
fn actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing || state.session.read_only();
    let busy = state.trial.busy();
    let unfinished = state.unfinished_trial_edit().is_some();
    let applying = state.navigation.applying_preview();
    let page = state
        .session
        .model()
        .preview_page()
        .filter(|_| state.trial.visible);
    let continuing = page.is_some_and(can_continue) && !unfinished && !applying && !busy;
    let waiting = page.is_some_and(|page| !page.ended && !can_continue(page));
    let restart_primary = !continuing && !waiting;
    let configuring = state.phase() != Phase::Writing;
    flex_row((
        continuing.then(|| {
            // After a step whose Continue goes away, focus moves to the next Continue or reply.
            print::primary_returning(
                scale,
                "Continue preview",
                &state.palette,
                (closing, true),
                |state: &mut Writer| {
                    let result = state.session.advance_preview(None);
                    state.report_trial_step(result);
                },
            )
        }),
        print::emphasised_action(
            scale,
            if applying {
                "Applying edit…"
            } else if unfinished {
                "Apply edit and preview"
            } else {
                state
                    .trial
                    .submission_caption(state.session.model().preview_page().is_some())
            },
            &state.palette,
            restart_primary || unfinished,
            closing
                || busy
                || state.navigation.busy()
                || state.work.busy()
                || state.localisation.preview_blocked()
                || state.reading.script().is_err()
                || state.reading.script().is_ok_and(<[_]>::is_empty),
            move |state: &mut Writer| {
                if unfinished {
                    state.apply_trial_edit()
                } else {
                    state.start_trial()
                }
            },
        ),
        (busy || applying).then(|| {
            text_button(
                scale,
                "Cancel preparation",
                Writer::cancel_trial_preparation,
            )
            .disabled(closing || (!applying && !state.trial.can_cancel()))
        }),
        print::action(
            scale,
            "Configure preview",
            configuring,
            Writer::configure_trial,
        ),
        context::toggle(state),
        trace::toggle(state, closing),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
}
