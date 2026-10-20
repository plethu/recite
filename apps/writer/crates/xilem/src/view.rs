use super::{Phase, Writer, keyed::keyed};
use crate::presentation::{label, paragraph, prose, text_button};
use recite_writer_model::{View, Workbench};
use xilem::core::one_of::Either;
use xilem::masonry::layout::AsUnit;
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, flex_col, flex_row, portal, sized_box, text_input};
use xilem::{InsertNewline, WidgetView};

mod manuscript;
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
use document_actions::{close_prompt, header, recovery_copy};

pub(super) fn writer(state: &mut Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    let scale = state.ui_scale();
    xilem::core::fork(
        portal(
            sized_box(
                flex_col((
                    header(state),
                    paragraph(
                        scale,
                        state.status.clone(),
                        state.script_width() + state.navigation_width() + 20.,
                    ),
                    state.external.path().map(|path| {
                        let path = state
                            .session
                            .project()
                            .and_then(|project| path.strip_prefix(project.root()).ok())
                            .unwrap_or(path);
                        paragraph(
                            scale,
                            format!(
                                "Editor request active for {}. You can keep writing.",
                                path.display()
                            ),
                            state.script_width() + state.navigation_width() + 20.,
                        )
                    }),
                    state.navigation.busy().then(|| {
                        text_button(scale, "Cancel pending action", Writer::cancel_navigation)
                    }),
                    recovery_copy(state),
                    close_prompt(state),
                    flex_row((
                        navigation(state),
                        flex_col((manuscript(state), trial::panel(state)))
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .gap(20.px()),
                    ))
                    .gap(20.px()),
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
                                scale,
                                format!("Recovery failed: {error}. Save or keep the window open."),
                            )
                        }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(16.px()),
            )
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
