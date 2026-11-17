//! The preview sheet's head: the title in Display with the run's captured context as its caption,
//! the run's actions, and the close icon back to writing. It stays put above the transcript.
use super::*;
use crate::app::keyboard::SlipAnchor;
use crate::presentation::control_style::ControlStyle;
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
        .map(|snapshot| ArcStr::from(snapshot.summary.clone()))
        // Before a run, its status: the head keeps one caption line in every state, so the
        // actions never move as a run starts.
        .or_else(|| super::status(state).map(ArcStr::from));
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
                    link: Writer::copy_link,
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
        // Always in the row, unavailable while the run cannot go on, so no action moves as a run
        // starts, waits on a reply or ends. After a step that leaves it unavailable, focus moves
        // to the next reply.
        {
            print::primary_returning(
                scale,
                // The sheet's own head says preview; the name keeps it for assistive technology.
                print::Title::named("Continue", "Continue preview"),
                &state.palette,
                (closing || !continuing, true),
                |state: &mut Writer| {
                    let result = state.session.advance_preview(None);
                    state.report_trial_step(result);
                },
            )
        },
        print::emphasised_action(
            scale,
            if applying {
                "Applying edit…".into()
            } else if unfinished {
                "Apply edit and preview".into()
            } else {
                let name = state
                    .trial
                    .submission_caption(state.session.model().preview_page().is_some());
                print::Title::named(name.strip_suffix(" preview").unwrap_or(name), name)
            },
            &state.palette,
            // Quiet unless it is the run's next step, so Continue and the replies lead.
            (restart_primary || unfinished, ControlStyle::Action),
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
        // Its slip hangs from under it while open.
        sized_box(print::action_named(
            scale,
            ("Configure…", "Configure preview"),
            configuring,
            Writer::configure_trial,
        ))
        .prop(SlipAnchor(state.configuring_trial())),
        context::toggle(state),
        trace::toggle(state, closing),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
}
