use super::{Phase, Writer, keyed::keyed};
use crate::presentation::{label, paragraph, prose, text_button};
use recite_writer_model::View;
use xilem::core::one_of::Either;
use xilem::masonry::layout::AsUnit;
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, flex_col, flex_row, portal, sized_box, text_input};
use xilem::{InsertNewline, WidgetView};

mod manuscript;
mod surface;
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Manuscript(pub bool);
impl xilem::masonry::core::UsesProperty<Manuscript> for xilem::masonry::widgets::TextInput {}
impl xilem::masonry::core::Property for Manuscript {
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

pub(super) fn writer(state: &Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    let surface = surface::Surface::resolve(state);
    let content = flex_col((
        header(state),
        paragraph(
            state.ui_scale(),
            state.status.clone(),
            state.script_width() + state.navigation_width() + 20.,
        ),
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
        matches!(surface, surface::Surface::SourceRepair { import: true, .. })
            .then(|| source_repair::panel(state)),
        workspace(state, &surface),
        recovery_failure(state),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(16.px());
    xilem::core::fork(
        portal(
            sized_box(content)
                .padding(24.px())
                .background_color(state.palette.canvas),
        )
        .constrain_horizontal(true),
        (
            crate::trial::worker(),
            crate::editor_work::worker(),
            crate::external_editor::worker(),
            crate::navigation::worker(),
        ),
    )
}
fn workspace(state: &Writer, surface: &surface::Surface<'_>) -> impl WidgetView<Writer> + use<> {
    flex_row((
        navigation(state),
        flex_col((manuscript(state, surface), trial::panel(state)))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(20.px()),
    ))
    .gap(20.px())
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
