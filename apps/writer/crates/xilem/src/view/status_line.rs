//! The status line: whether the work is saved, the problem count as a way to the first problem,
//! the latest status sentence on one line, and the actions a running command offers.
use super::*;
use crate::presentation::Type;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::print;
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use xilem::masonry::properties::Dimensions;
use xilem::view::button;

/// The status line's height: one Meta line and its padding.
pub(super) fn status_height(scale: f32) -> f64 {
    2. * space::UNIT + f64::from(Type::Meta.metrics().1 * scale)
}

pub(super) fn status_line(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let dirty = state.session.current_dirty() || state.session.model().has_draft();
    let (text, signal) = (state.palette.status_text, state.palette.status_signal);
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    let refusal = state.reading_refusal();
    let saved = if state.session.project_root().is_none() {
        "Temporary example"
    } else if dirty {
        "Unsaved changes"
    } else if !state.notices.found.is_empty() {
        // Under the disk-change banner the saved copy is no longer what is on disk.
        "Changed on disk"
    } else {
        "Saved to disk"
    };
    flex_row((
        print::meta(state.ui_scale(), saved).color(text),
        print::separator(state.ui_scale(), text),
        if let (None, false, Some(count)) = (
            refusal,
            state.unfinished_trial_edit().is_some() && state.trial.visible,
            (!diagnostics.is_empty()).then_some(diagnostics.len()),
        ) {
            // The count is a way to the first problem: focus moves to its wedge.
            Either::A(styled(
                ControlStyle::Status,
                button(
                    print::meta(
                        state.ui_scale(),
                        format!("{count} problem{}", if count == 1 { "" } else { "s" }),
                    )
                    .color(text),
                    Writer::reveal_first_problem,
                ),
            ))
        } else {
            Either::B(
                print::meta(
                    state.ui_scale(),
                    if let Some(refusal) = refusal {
                        refusal.footer().to_owned()
                    } else if state.unfinished_trial_edit().is_some() && state.trial.visible {
                        "Preview needs an applied edit".to_owned()
                    } else {
                        "no problems".to_owned()
                    },
                )
                .color(
                    if refusal.is_none() && state.unfinished_trial_edit().is_none() {
                        signal
                    } else {
                        text
                    },
                ),
            )
        },
        // A status sentence takes the room: the Split note returns once it is read.
        (state.workspaces.screen == crate::workspaces::Screen::Write
            && !workspace::Layout::resolve(state).split_available
            && matches!(state.status.as_str(), "Ready" | "Map ready"))
        .then(|| {
            (
                print::separator(state.ui_scale(), text),
                print::meta(state.ui_scale(), "Split needs a wider window").color(text),
            )
        }),
        (!matches!(state.status.as_str(), "Ready" | "Map ready")).then(|| {
            (
                print::separator(state.ui_scale(), text),
                // One line, ending in an ellipsis where it would not fit; read in full.
                one_line(state.status.clone(), Type::Meta, state.ui_scale(), text).flex(1.),
            )
        }),
        // A beat renamed in place reports here, with its Undo while the report stands.
        state.offers_rename_undo().then(|| {
            styled(
                ControlStyle::Status,
                button(
                    print::meta(state.ui_scale(), "Undo rename").color(text),
                    Writer::undo_project_rename,
                ),
            )
        }),
        // A build runs as a command: its progress is the status sentence, and it cancels here.
        state
            .session
            .project()
            .is_some_and(|project| project.files().builds.busy())
            .then(|| {
                styled(
                    ControlStyle::Status,
                    button(
                        print::meta(state.ui_scale(), "Cancel build").color(text),
                        Writer::cancel_build,
                    ),
                )
            }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap((space::SIBLINGS * f64::from(state.ui_scale())).px())
    // One line and its padding tall, whether or not the problem count is a button.
    .padding(xilem::style::Padding::from_vh(0.px(), space::KINDS.px()))
    .dims(Dimensions::AUTO.with_height(status_height(state.ui_scale()).px()))
    .background_color(state.palette.status_bar)
}

#[cfg(test)]
mod tests;
