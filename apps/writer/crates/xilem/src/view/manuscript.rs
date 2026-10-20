use super::*;

pub(super) fn manuscript(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    let model = state.project.current().model();
    let editing = matches!(model.view(), View::Passage(_));
    let editor = if !editing {
        Either::A(super::reading::inspection(state))
    } else {
        Either::B(
            sized_box(keyed(
                (model.document().key().clone(), model.view().clone()),
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
            ))
            .width(560.px())
            .height(if state.trial.visible {
                100.px()
            } else {
                260.px()
            }),
        )
    };
    let selection = if editing {
        "Manuscript"
    } else {
        "Beat reading"
    };
    flex_col((
        label(selection),
        actions(state),
        editor,
        diagnostics(state),
        flex_row((
            text_button("Apply draft", |state: &mut Writer| {
                state.change(Workbench::apply)
            })
            .disabled(closing || !editing),
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
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase != Phase::Writing;
    let repair = state.reading.script().is_err();
    let selected = state.reading.selected.is_some() && state.reading.error.is_none() && !repair;
    flex_row((
        text_button("Add beat", |state: &mut Writer| {
            state.change(Workbench::add_beat)
        })
        .disabled(closing || repair),
        text_button("Add line", |state: &mut Writer| {
            state.change(Workbench::add_line)
        })
        .disabled(closing || !selected),
        text_button("Add reply", |state: &mut Writer| {
            state.change(Workbench::add_choice)
        })
        .disabled(closing || !selected),
        text_button("Read beat", Writer::read_current).disabled(closing || !selected),
    ))
    .gap(8.px())
}
