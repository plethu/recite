use super::{Phase, Writer, keyed::keyed};
use recite_writer_model::{View, Workbench};
use xilem::core::one_of::Either;
use xilem::masonry::layout::AsUnit;
use xilem::peniko::Color;
use xilem::style::Style;
use xilem::view::{
    CrossAxisAlignment, flex_col, flex_row, label, portal, prose, sized_box, text_button,
    text_input,
};
use xilem::{InsertNewline, WidgetView};

mod manuscript;
mod navigation;
mod reading;
use manuscript::manuscript;
mod trial;
use navigation::navigation;

pub(super) fn writer(state: &mut Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    xilem::core::fork(
        portal(
            sized_box(
                flex_col((
                    header(state),
                    close_prompt(state),
                    flex_row((
                        navigation(state),
                        flex_col((manuscript(state), trial::panel(state)))
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .gap(20.px()),
                    ))
                    .gap(20.px()),
                    sized_box(prose(state.status.clone())).width(830.px()),
                    state
                        .project
                        .sessions()
                        .find_map(|session| session.recovery_error())
                        .map(|error| {
                            label(format!(
                                "Recovery failed: {error}. Save or keep the window open."
                            ))
                        }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(16.px()),
            )
            .padding(24.px())
            .background_color(Color::from_rgb8(27, 24, 30)),
        )
        .constrain_horizontal(true),
        crate::trial::worker(),
    )
}

fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    flex_col((
        label("Recite Writer").text_size(28.),
        label(format!(
            "{}{}",
            state.project.current().model().document().key().as_str(),
            if state.project.current().dirty() {
                " · Unsaved"
            } else {
                ""
            }
        )),
        flex_row((
            text_button("Save file", |state: &mut Writer| {
                state.save();
            })
            .disabled(closing),
            text_button("Save all", |state: &mut Writer| {
                state.save_all();
            })
            .disabled(closing),
            text_button("Open in editor", Writer::open_editor)
                .disabled(closing || state.editor.is_none()),
            text_button("Reload (keep recovery copy)", Writer::reload).disabled(closing),
            text_button("Close", Writer::request_close).disabled(closing),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn diagnostics(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let diagnostics = state
        .project
        .current()
        .model()
        .document()
        .diagnostic_snapshot();
    flex_col(
        (0..diagnostics.len().min(3))
            .filter_map(|index| diagnostics.get(index))
            .map(|diagnostic| {
                sized_box(prose(format!(
                    "Source diagnostics · line {} · {}",
                    diagnostic.span.start.line(),
                    diagnostic.message
                )))
                .width(560.px())
            })
            .collect::<Vec<_>>(),
    )
    .gap(6.px())
}

fn close_prompt(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    (state.phase == Phase::ConfirmClose).then(|| {
        sized_box(
            flex_col((
                label("Keep your unsaved work before closing"),
                flex_row((
                    text_button("Keep editing", |state: &mut Writer| {
                        state.phase = Phase::Writing
                    }),
                    text_button("Save and close", |state: &mut Writer| {
                        if state.save_all() {
                            state.phase = Phase::Closed;
                        }
                    }),
                    text_button("Close and keep recovery", Writer::close_with_recovery),
                ))
                .gap(8.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .padding(12.px())
        .background_color(Color::from_rgb8(45, 39, 49))
    })
}
