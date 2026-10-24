//! Compact writing tools share the reference shell; narrow windows reflow by groups.
use super::*;
use crate::app::{Request, picker::Search};
use crate::navigation::Target;
use crate::presentation::{Icon, print};
use recite_writer_model::EditOperation;
use recite_writer_session::choices::display_name;
use xilem::{
    masonry::properties::LineBreaking,
    view::{GridExt, GridTrackSize, grid},
};

pub(super) fn toolbar(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = (state.viewport.width - state.navigation_width() - 32.).max(280.);
    let wide = width >= 1120. * f64::from(state.ui_scale());
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    grid((
        history(state).grid_pos(0, 0),
        workspace::workspace_tabs(state).grid_pos(u16::from(wide), u16::from(!wide)),
        document_tools(state, blocked).grid_pos(if wide { 2 } else { 1 }, u16::from(!wide)),
        app_tools(state, blocked).grid_pos(if wide { 3 } else { 1 }, 0),
    ))
    .columns(if wide {
        [
            GridTrackSize::FRACTION,
            GridTrackSize::Auto,
            GridTrackSize::Auto,
            GridTrackSize::Auto,
        ]
    } else {
        [
            GridTrackSize::FRACTION,
            GridTrackSize::Auto,
            GridTrackSize::Fixed(0.px()),
            GridTrackSize::Fixed(0.px()),
        ]
    })
    .rows(vec![GridTrackSize::Auto; if wide { 1 } else { 2 }])
    .gap(8.px())
    .padding(xilem::style::Padding::from_vh(4.px(), 16.px()))
    .background_color(state.palette.surface)
}
fn history(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
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
fn app_tools(state: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::action(scale, "Commands", blocked, |state: &mut Writer| {
            state.app_request = Some(Request::Search(Search::Commands))
        }),
        (state.viewport.width - state.navigation_width() >= 600. * f64::from(scale))
            .then(|| workspace::authoring_modes(state)),
        text_button(scale, "Workspace", |state: &mut Writer| {
            state.app_request = Some(Request::Workspace)
        })
        .disabled(blocked),
    ))
    .gap(8.px())
}
fn document_tools(state: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let history = state.viewport.width - state.navigation_width() >= 600. * f64::from(scale);
    let editable = matches!(state.session.model().view(), View::Passage(_))
        || state
            .rules
            .as_ref()
            .is_some_and(|rules| rules.is_open_for(state.session.model()));
    flex_row((
        print::action(scale, "Try scene", blocked, Writer::start_trial),
        (editable && state.session.model().has_draft()).then(|| {
            print::action(
                scale,
                "Apply draft",
                blocked || state.session.read_only(),
                |state: &mut Writer| state.navigate(Target::Edit(EditOperation::Apply)),
            )
        }),
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
        print::action(scale, "Project", blocked, |state: &mut Writer| {
            state.document_menu.toggle()
        }),
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
        print::meta(state.ui_scale(), format!("{saved} ·")).color(state.palette.status_text),
        print::meta(
            state.ui_scale(),
            if diagnostics.is_empty() {
                "no problems".to_owned()
            } else {
                format!("{} problems", diagnostics.len())
            },
        )
        .color(if diagnostics.is_empty() {
            state.palette.status_signal
        } else {
            state.palette.status_text
        }),
        (!matches!(state.status.as_str(), "Ready" | "Map ready")).then(|| {
            print::meta(state.ui_scale(), state.status.clone())
                .color(state.palette.status_text)
                .prop(LineBreaking::WordWrap)
                .flex(1.)
        }),
    ))
    .gap(8.px())
    .padding(xilem::style::Padding::from_vh(6.px(), 16.px()))
    .background_color(state.palette.status_bar)
}
