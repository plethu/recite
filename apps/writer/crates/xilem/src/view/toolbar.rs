//! Compact writing tools share the reference shell; narrow windows reflow by groups.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::space;
use crate::presentation::tokens::breakpoint;
use crate::presentation::{Icon, print};
use crate::presentation::{Type, Typeset};
use recite_writer_session::choices::display_name;
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::Dimensions;
use xilem::view::FlexSpacer;
use xilem::{
    masonry::properties::LineBreaking,
    view::{GridExt, GridTrackSize, button, grid},
};

/// One row from about 600 logical pixels: history, workspace modes, then the scene's
/// tools with Save as the only raised action. Narrower windows move the modes to a second row.
pub(super) fn toolbar(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = state.content_width();
    let wide = width >= breakpoint::TOOLBAR * f64::from(state.ui_scale());
    let blocked = state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy();
    let padding = xilem::style::Padding::from_vh(space::UNIT.px(), space::KINDS.px());
    // Tool screens have no workspaces: history, then the tools, on one row where they fit.
    if state.workspaces.screen != crate::workspaces::Screen::Write {
        return Either::A(
            flex_row((history(state).flex(1.), document_tools(state, blocked)))
                .direction(if wide {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                })
                .cross_axis_alignment(if wide {
                    CrossAxisAlignment::Center
                } else {
                    CrossAxisAlignment::End
                })
                .gap(space::SIBLINGS.px())
                .padding(padding),
        );
    }
    Either::B(
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
        .gap(space::SIBLINGS.px())
        .padding(padding),
    )
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
        // The galley's beat heading carries the link; wherever no heading shows, the toolbar does.
        (state.visible_workspace() == crate::map::Workspace::Map
            || state.trial.visible
            || state.workspaces.screen != crate::workspaces::Screen::Write
            || !super::manuscript::galley(state, &surface::Surface::resolve(state)))
        .then(|| {
            print::icon_action(
                scale,
                "Copy link",
                Icon::Link,
                &state.palette,
                blocked,
                Writer::copy_link,
            )
        }),
        (state.visible_workspace() == crate::map::Workspace::Map).then(|| {
            label(scale, display_name(key.rsplit('/').next().unwrap_or(key)))
                .typeset(Type::Section, scale)
                .prop(LineBreaking::WordWrap)
                .flex(1.)
        }),
        FlexSpacer::Flex(1.),
    ))
    .gap(space::UNIT.px())
}
fn document_tools(state: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let history =
        state.viewport.width - state.navigation_width() >= breakpoint::TOOLBAR * f64::from(scale);
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
        // Save is tonal while clean and the view's one primary with unsaved work.
        state.session.project().map(|_| {
            print::emphasised_action(
                scale,
                "Save",
                &state.palette,
                state.save_is_primary(),
                blocked || state.localisation.saving() || state.source_repair_pending(),
                Writer::save,
            )
        }),
    ))
    .gap(space::UNIT.px())
}

/// The status line's height: one Meta line and its padding.
pub(super) fn status_height(scale: f32) -> f64 {
    2. * space::UNIT + f64::from(Type::Meta.metrics().1 * scale)
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
                    .color(state.palette.status_text),
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
                        state.palette.status_signal
                    } else {
                        state.palette.status_text
                    },
                ),
            )
        },
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
        // A build runs as a command: its progress is the status sentence, and it cancels here.
        state
            .session
            .project()
            .is_some_and(|project| project.files().builds.busy())
            .then(|| {
                styled(
                    ControlStyle::Status,
                    button(
                        print::meta(state.ui_scale(), "Cancel build")
                            .color(state.palette.status_text),
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
