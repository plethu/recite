//! Compact writing tools share the reference shell; narrow windows reflow by groups.
use super::*;
use crate::presentation::{Icon, print};
use recite_writer_session::choices::display_name;
use xilem::{
    masonry::properties::LineBreaking,
    view::{GridExt, GridTrackSize, grid},
};

/// One row from about 760 logical pixels: title and history, workspace modes, then the scene's
/// tools with Save as the only raised action. Narrower windows move the modes to a second row.
pub(super) fn toolbar(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = (state.viewport.width - state.navigation_width() - 32.).max(280.);
    let wide = width >= 760. * f64::from(state.ui_scale());
    let blocked = state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy();
    grid((
        history(state).grid_pos(0, 0),
        workspace::workspace_tabs(state).grid_pos(u16::from(wide), u16::from(!wide)),
        document_tools(state, blocked).grid_pos(if wide { 2 } else { 1 }, 0),
    ))
    .columns(if wide {
        [
            GridTrackSize::FRACTION,
            GridTrackSize::Auto,
            GridTrackSize::Auto,
        ]
    } else {
        [
            GridTrackSize::FRACTION,
            GridTrackSize::Auto,
            GridTrackSize::Fixed(0.px()),
        ]
    })
    .rows(vec![GridTrackSize::Auto; if wide { 1 } else { 2 }])
    .gap(8.px())
    .padding(xilem::style::Padding::from_vh(4.px(), 16.px()))
}
fn history(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let blocked = state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy();
    let key = state.session.model().document().key().as_str();
    flex_row((
        print::icon_action(
            scale,
            "Back",
            Icon::Back,
            &state.palette,
            blocked || state.navigation.history.step(false).is_none(),
            |state: &mut Writer| state.history_step(false),
        ),
        print::icon_action(
            scale,
            "Forward",
            Icon::Forward,
            &state.palette,
            blocked || state.navigation.history.step(true).is_none(),
            |state: &mut Writer| state.history_step(true),
        ),
        print::icon_action(
            scale,
            "Copy link",
            Icon::Link,
            &state.palette,
            blocked,
            Writer::copy_link,
        ),
        label(scale, display_name(key.rsplit('/').next().unwrap_or(key)))
            .text_size(18. * scale)
            .prop(LineBreaking::WordWrap)
            .flex(1.),
    ))
    .gap(4.px())
}
fn document_tools(state: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let history = state.viewport.width - state.navigation_width() >= 600. * f64::from(scale);
    flex_row((
        print::action(scale, "Try scene", blocked, Writer::start_trial),
        history.then(|| {
            print::icon_action(
                scale,
                "Undo applied",
                Icon::Undo,
                &state.palette,
                blocked,
                |state: &mut Writer| state.undo_redo(false),
            )
        }),
        history.then(|| {
            print::icon_action(
                scale,
                "Redo applied",
                Icon::Redo,
                &state.palette,
                blocked,
                |state: &mut Writer| state.undo_redo(true),
            )
        }),
        // Workspace holds the project, document and app actions in one menu.
        print::action(scale, "Workspace", blocked, Writer::toggle_document_menu),
        state.session.project().map(|_| {
            text_button(scale, "Save", Writer::save)
                .disabled(blocked || state.localisation.saving() || state.source_repair_pending())
        }),
    ))
    .gap(4.px())
}

pub(super) fn status(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let dirty = state.session.current_dirty() || state.session.model().has_draft();
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    let refusal = state.reading_refusal();
    let saved = if state.session.project_root().is_none() {
        "Temporary example"
    } else if dirty {
        "Unsaved changes"
    } else {
        "Saved to disk"
    };
    flex_row((
        print::meta(state.ui_scale(), saved).color(state.palette.status_text),
        print::separator(state.ui_scale(), state.palette.status_text),
        print::meta(
            state.ui_scale(),
            if let Some(refusal) = refusal {
                refusal.footer().to_owned()
            } else if state.unfinished_trial_edit().is_some() && state.trial.visible {
                "Preview needs an applied edit".to_owned()
            } else if diagnostics.is_empty() {
                "no problems".to_owned()
            } else {
                format!(
                    "{} problem{}",
                    diagnostics.len(),
                    if diagnostics.len() == 1 { "" } else { "s" }
                )
            },
        )
        .color(
            if diagnostics.is_empty()
                && refusal.is_none()
                && state.unfinished_trial_edit().is_none()
            {
                state.palette.status_signal
            } else {
                state.palette.status_text
            },
        ),
        (state.workspaces.screen == crate::workspaces::Screen::Write
            && !workspace::Layout::resolve(state).split_available)
            .then(|| {
                (
                    print::separator(state.ui_scale(), state.palette.status_text),
                    print::meta(state.ui_scale(), "Split needs a wider window")
                        .color(state.palette.status_text),
                )
            }),
        (!matches!(state.status.as_str(), "Ready" | "Map ready")).then(|| {
            (
                print::separator(state.ui_scale(), state.palette.status_text),
                print::meta(state.ui_scale(), state.status.clone())
                    .color(state.palette.status_text)
                    .prop(LineBreaking::WordWrap)
                    .flex(1.),
            )
        }),
    ))
    .gap((8. * f64::from(state.ui_scale())).px())
    .padding(xilem::style::Padding::from_vh(6.px(), 16.px()))
    .background_color(state.palette.status_bar)
}

#[cfg(test)]
mod tests;
