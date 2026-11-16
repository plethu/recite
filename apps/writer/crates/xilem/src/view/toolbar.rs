//! Compact writing tools share the reference shell; narrow windows reflow by groups.
use super::*;
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::breakpoint;
use crate::presentation::{Icon, Type, print};
use recite_writer_session::choices::display_name;
use xilem::masonry::kurbo::Axis;
use xilem::view::FlexSpacer;
use xilem::view::{GridExt, GridTrackSize, grid};

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
        // One line ending in an ellipsis where the toolbar is short of room; the whole name is
        // still read aloud.
        (state.visible_workspace() == crate::map::Workspace::Map).then(|| {
            one_line(
                display_name(key.rsplit('/').next().unwrap_or(key)),
                Type::Section,
                scale,
                state.palette.ink,
            )
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
