//! Writing chrome reflows in stable slots while the document scrolls independently.
use super::*;
use crate::presentation::print;
use recite_writer_session::choices::display_name;
use xilem::{
    masonry::properties::LineBreaking,
    view::{FlexSpacer, GridExt, GridParams, GridTrackSize, grid},
};

pub(super) fn toolbar(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = (state.viewport.width - state.navigation_width() - 32.).max(280.);
    let map = state.visible_workspace() == crate::map::Workspace::Map;
    let compact = width < 900. * f64::from(state.ui_scale());
    let wide = map || width >= 600. * f64::from(state.ui_scale());
    let left = if wide {
        width * if compact { 0.65 } else { 0.5 } - 2.
    } else {
        width
    };
    let right = if wide { width - left - 4. } else { width };
    flex_col((grid((
        history(state).grid_pos(0, 0),
        file_actions(state, compact).grid_pos(u16::from(wide), u16::from(!wide)),
        workspace::workspace_tabs(state).grid_params(
            GridParams::pos(0, if wide { 1 } else { 2 }).with_width(if map { 2 } else { 1 }),
        ),
        retained(editing(state, right, compact), !map)
            .grid_pos(u16::from(wide), if wide { 1 } else { 3 }),
    ))
    .columns(if map {
        let actions = 180. * f64::from(state.ui_scale());
        [
            GridTrackSize::Fixed((width - actions - 4.).px()),
            GridTrackSize::Fixed(actions.px()),
        ]
    } else {
        [
            GridTrackSize::Fixed(left.px()),
            GridTrackSize::Fixed(if wide { right.px() } else { 0.px() }),
        ]
    })
    .rows(vec![GridTrackSize::Auto; if wide { 2 } else { 4 }])
    .gap(4.px()),))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(4.px())
    .padding(xilem::style::Padding::from_vh(8.px(), 16.px()))
    .background_color(state.palette.surface)
}
fn history(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    let key = state.session.model().document().key().as_str();
    flex_row((
        print::action(
            scale,
            "Back",
            blocked || state.navigation.history.step(false).is_none(),
            |state: &mut Writer| state.history_step(false),
        ),
        print::action(
            scale,
            "Forward",
            blocked || state.navigation.history.step(true).is_none(),
            |state: &mut Writer| state.history_step(true),
        ),
        label(scale, display_name(key.rsplit('/').next().unwrap_or(key)))
            .text_size(18. * scale)
            .prop(LineBreaking::WordWrap)
            .flex(1.),
        document_actions::copy_link(state),
    ))
    .gap(4.px())
}
fn file_actions(state: &Writer, compact: bool) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    flex_row((
        FlexSpacer::Flex(1.),
        state
            .session
            .project()
            .filter(|_| state.visible_workspace() != crate::map::Workspace::Map)
            .map(|_| {
                text_button(
                    state.ui_scale(),
                    if compact { "Save" } else { "Save file" },
                    Writer::save,
                )
                .disabled(blocked || state.localisation.saving() || state.source_repair_pending())
            }),
        print::action(
            state.ui_scale(),
            "Document actions ⋯",
            blocked,
            |state: &mut Writer| state.document_menu.toggle(),
        ),
    ))
    .gap(4.px())
}
fn editing(state: &Writer, width: f64, compact: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    let editable = matches!(state.session.model().view(), View::Passage(_))
        || state
            .rules
            .as_ref()
            .is_some_and(|reply| reply.is_open_for(state.session.model()));
    let wide = width >= 500. * f64::from(scale);
    grid((
        print::action(scale, "Try scene", blocked, Writer::start_trial).grid_pos(0, 0),
        print::action(
            scale,
            "Apply draft",
            blocked || !editable || !state.session.model().has_draft() || state.session.read_only(),
            |state: &mut Writer| {
                state.navigate(crate::navigation::Target::Edit(
                    recite_writer_model::EditOperation::Apply,
                ))
            },
        )
        .grid_pos(1, 0),
        retained(
            print::action(scale, "Undo applied", blocked, |state: &mut Writer| {
                state.undo_redo(false)
            }),
            !compact,
        )
        .grid_pos(if wide { 2 } else { 0 }, u16::from(!wide)),
        retained(
            print::action(scale, "Focus writing", blocked, Writer::toggle_focus),
            !compact,
        )
        .grid_pos(if wide { 3 } else { 1 }, u16::from(!wide)),
    ))
    .columns([GridTrackSize::Auto; 4])
    .rows(vec![GridTrackSize::Auto; if wide { 1 } else { 2 }])
    .gap(4.px())
}

pub(super) fn status(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let dirty = state.session.current_dirty() || state.session.model().has_draft();
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    let saved = if state.session.project_root().is_none() {
        "Temporary example"
    } else if state.session.model().has_draft() {
        "Unapplied changes"
    } else if dirty {
        "Unsaved changes"
    } else {
        "Saved to disk"
    };
    flex_row((
        label(state.ui_scale(), format!("{saved} ·"))
            .text_size(12. * state.ui_scale())
            .color(state.palette.status_text),
        label(
            state.ui_scale(),
            if diagnostics.is_empty() {
                "no problems".to_owned()
            } else {
                format!("{} problems", diagnostics.len())
            },
        )
        .text_size(12. * state.ui_scale())
        .color(if diagnostics.is_empty() {
            state.palette.status_signal
        } else {
            state.palette.status_text
        }),
        label(state.ui_scale(), state.status.clone())
            .text_size(12. * state.ui_scale())
            .color(state.palette.status_text)
            .prop(LineBreaking::WordWrap)
            .flex(1.),
    ))
    .gap(8.px())
    .padding(xilem::style::Padding::from_vh(6.px(), 16.px()))
    .background_color(state.palette.status_bar)
}
