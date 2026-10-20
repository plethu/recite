use super::{Phase, Writer, keyed::keyed};
use crate::presentation::{label, paragraph, prose, text_button};
use recite_writer_model::View;
use xilem::core::one_of::Either;
use xilem::masonry::core;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::widgets;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, flex_col, flex_row, indexed_stack, portal, sized_box, text_input,
};
use xilem::{InsertNewline, WidgetView};

mod manuscript;
mod surface;
pub(super) mod workspace;
use workspace::workspace;
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Manuscript(pub bool);
impl core::UsesProperty<Manuscript> for widgets::TextInput {}
impl core::Property for Manuscript {
    fn static_default() -> &'static Self {
        static OTHER: Manuscript = Manuscript(false);
        &OTHER
    }
}
mod navigation;
mod reading;
use manuscript::manuscript;
mod trial;
use navigation::navigation;
mod document_actions;
mod source_repair;
use document_actions::{close_prompt, header, recovery_copy};

fn retained<V: WidgetView<Writer>>(child: V, visible: bool) -> impl WidgetView<Writer> {
    indexed_stack((child, sized_box(label(1., "")).dims((0.px(), 0.px()))))
        .active(usize::from(!visible))
}

pub(super) fn writer(state: &Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    let surface = surface::Surface::resolve(state);
    let content = flex_col((
        indexed_stack((
            header(state),
            document_actions::focus_header(state),
            document_actions::project_header(state),
        ))
        .active(
            if state.workspaces.screen != crate::workspaces::Screen::Write {
                2
            } else {
                usize::from(state.attention.is_focused())
            },
        ),
        state.session.recovery_reason().is_none().then(|| {
            paragraph(
                state.ui_scale(),
                state.status.clone(),
                state.script_width() + state.navigation_width() + 20.,
            )
        }),
        crate::workspaces::recovery(state),
        state.session.checked_out().then(|| {
            text_button(
                state.ui_scale(),
                "Cancel project action",
                Writer::cancel_project_action,
            )
        }),
        state.localisation.working().then(|| {
            text_button(
                state.ui_scale(),
                "Cancel catalogue action",
                Writer::cancel_catalogue_action,
            )
        }),
        editor_status(state),
        state.navigation.busy().then(|| {
            text_button(
                state.ui_scale(),
                "Cancel pending action",
                Writer::cancel_navigation,
            )
        }),
        recovery_copy(state),
        close_prompt(state),
        state
            .session
            .project()
            .is_some_and(|project| project.files().builds.busy())
            .then(|| text_button(state.ui_scale(), "Cancel build", Writer::cancel_build)),
        state
            .declaration_session()
            .is_some_and(|session| session.generation_active())
            .then(|| {
                text_button(
                    state.ui_scale(),
                    "Cancel schema generation",
                    Writer::cancel_schema_generation,
                )
            }),
        (state.workspaces.screen == crate::workspaces::Screen::Write
            && state.visible_workspace() != crate::map::Workspace::Map
            && matches!(surface, surface::Surface::SourceRepair { import: true, .. }))
        .then(|| source_repair::panel(state)),
        workspace::workspace_tabs(state),
        xilem::view::indexed_stack((workspace(state, &surface), crate::workspaces::panel(state)))
            .active(usize::from(
                state.workspaces.screen != crate::workspaces::Screen::Write,
            )),
        recovery_failure(state),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(16.px());
    xilem::core::fork(
        xilem::view::resize_observer(
            |state: &mut Writer, size| state.viewport = size,
            portal(
                sized_box(content)
                    .padding(24.px())
                    .background_color(state.palette.canvas),
            )
            .constrain_horizontal(true),
        ),
        (
            crate::trial::worker(),
            crate::editor_work::worker(),
            crate::external_editor::worker(),
            crate::navigation::worker(),
            crate::workspaces::worker(),
            crate::workspaces::jobs(state),
        ),
    )
}
fn editor_status(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    state.external.path().map(|path| {
        let path = state
            .session
            .project()
            .and_then(|project| path.strip_prefix(project.root()).ok())
            .unwrap_or(path);
        paragraph(
            state.ui_scale(),
            format!(
                "Editor request active for {}. You can keep writing.",
                path.display()
            ),
            state.script_width() + state.navigation_width() + 20.,
        )
    })
}
fn recovery_failure(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if state.session.recovery_reason().is_some() {
        return None;
    }
    state
        .session
        .project()
        .and_then(|project| {
            project
                .sessions()
                .find_map(|session| session.recovery_error())
        })
        .map(|error| {
            label(
                state.ui_scale(),
                format!("Recovery failed: {error}. Save or keep the window open."),
            )
        })
}
