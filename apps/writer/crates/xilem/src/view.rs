use super::{Phase, Writer};
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

const PAGE: usize = 8;

pub(super) fn writer(state: &mut Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    portal(
        sized_box(
            flex_col((
                header(state),
                close_prompt(state),
                flex_row((outline(state), manuscript(state))).gap(20.px()),
                diagnostics(state),
                sized_box(prose(state.status.clone())).width(830.px()),
                state.session.recovery_error().map(|error| {
                    label(format!(
                        "Recovery failed: {error}. Save or keep the window open."
                    ))
                }),
            ))
            .gap(16.px()),
        )
        .padding(24.px())
        .background_color(Color::from_rgb8(27, 24, 30)),
    )
    .constrain_horizontal(true)
}

fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    flex_col((
        label("Recite Writer").text_size(28.),
        label(format!(
            "{}{}",
            state.session.model().document().key().as_str(),
            if state.session.dirty() {
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
            text_button("Open in editor", Writer::open_editor)
                .disabled(closing || state.editor.is_none()),
            text_button("Reload (keep recovery copy)", Writer::reload).disabled(closing),
            text_button("Close", Writer::request_close).disabled(closing),
        ))
        .gap(8.px()),
    ))
    .gap(12.px())
}

fn outline(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    let model = state.session.model();
    let passages = model.document().passage_snapshot().unwrap_or_default();
    let pages = passages.len().div_ceil(PAGE).max(1);
    state.page = state.page.min(pages - 1);
    let entries = passages
        .iter()
        .skip(state.page * PAGE)
        .take(PAGE)
        .map(|passage| {
            let id = passage.id.clone();
            let current = model.view() == &View::Passage(id.clone());
            let excerpt: String = passage.text.chars().take(12).collect();
            text_button(
                format!("{}{}…", if current { "Current · " } else { "" }, excerpt),
                move |state: &mut Writer| state.select(id.clone()),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            label("Passages"),
            flex_col(entries).gap(8.px()),
            flex_row((
                text_button("Previous", |state: &mut Writer| {
                    state.page = state.page.saturating_sub(1)
                })
                .disabled(closing || state.page == 0),
                text_button("Next", |state: &mut Writer| state.page += 1)
                    .disabled(closing || state.page + 1 >= pages),
            ))
            .gap(8.px()),
            label(format!("Page {} of {pages}", state.page + 1)),
        ))
        .gap(12.px()),
    )
    .width(250.px())
}

fn manuscript(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    let model = state.session.model();
    let source_required = !matches!(model.view(), View::Passage(_));
    let editor = if source_required {
        Either::A(sized_box(prose("Edit this source in your editor. Its bytes are preserved. Open it externally, then reload from disk.")).width(560.px()))
    } else {
        Either::B(
            sized_box(
                text_input(model.draft().to_owned(), |state: &mut Writer, text| {
                    state.change(|model| {
                        model.set_draft(text);
                        Ok(())
                    });
                })
                .insert_newline(InsertNewline::OnEnter)
                .font("Literata")
                .text_size(20.)
                .placeholder("Manuscript")
                .disabled(closing)
                .prop(xilem::masonry::properties::TextAreaLabel::new("Manuscript")),
            )
            .width(560.px())
            .height(260.px()),
        )
    };
    let selection = if source_required {
        "Source editing in your editor"
    } else {
        "Manuscript"
    };
    flex_col((
        label(selection),
        editor,
        flex_row((
            text_button("Apply draft", |state: &mut Writer| {
                state.change(Workbench::apply)
            })
            .disabled(closing || source_required),
            text_button("Discard draft", |state: &mut Writer| {
                state.change(|model| {
                    model.discard();
                    Ok(())
                })
            })
            .disabled(closing),
            text_button("Undo applied", |state: &mut Writer| {
                state.change(Workbench::undo)
            })
            .disabled(closing),
            text_button("Redo applied", |state: &mut Writer| {
                state.change(Workbench::redo)
            })
            .disabled(closing),
        ))
        .gap(8.px()),
    ))
    .gap(12.px())
}

fn diagnostics(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    flex_col(
        (0..diagnostics.len().min(3))
            .filter_map(|index| diagnostics.get(index))
            .map(|diagnostic| {
                sized_box(prose(format!(
                    "Line {} · {}",
                    diagnostic.span.start.line(),
                    diagnostic.message
                )))
                .width(830.px())
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
                        if state.save() {
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
