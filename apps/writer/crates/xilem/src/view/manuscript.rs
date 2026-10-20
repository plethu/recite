use super::*;

pub(super) fn manuscript(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    let model = state.session.model();
    let editing = matches!(model.view(), View::Passage(_));
    let has_draft = model.has_draft();
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
                .text_size(state.reading_size())
                .placeholder("Manuscript")
                .disabled(closing)
                .prop(xilem::masonry::properties::TextAreaLabel::new("Manuscript"))
                .prop(Manuscript(true)),
            ))
            .dims((
                state.script_width().px(),
                if state.trial.visible {
                    100.px()
                } else {
                    260.px()
                },
            )),
        )
    };
    let selection = if editing {
        "Manuscript"
    } else {
        "Beat reading"
    };
    flex_col((
        label(scale, selection),
        actions(state),
        editor,
        diagnostics(state),
        flex_row((
            text_button(scale, "Apply draft", |state: &mut Writer| {
                state.change(Workbench::apply)
            })
            .disabled(closing || !editing || !has_draft),
            text_button(scale, "Discard draft", |state: &mut Writer| {
                state.change(|model| {
                    model.discard();
                    Ok(())
                })
            })
            .disabled(closing || !has_draft),
            text_button(scale, "Undo applied", |state: &mut Writer| {
                state.change(Workbench::undo)
            })
            .disabled(closing),
            text_button(scale, "Redo applied", |state: &mut Writer| {
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
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    let repair = state.reading.script().is_err();
    let selected = state.reading.selected.is_some() && state.reading.error.is_none() && !repair;
    flex_row((
        text_button(scale, "Add beat", |state: &mut Writer| {
            state.change(Workbench::add_beat)
        })
        .disabled(closing || repair),
        text_button(scale, "Add line", |state: &mut Writer| {
            state.change(Workbench::add_line)
        })
        .disabled(closing || !selected),
        text_button(scale, "Add reply", |state: &mut Writer| {
            state.change(Workbench::add_choice)
        })
        .disabled(closing || !selected),
        text_button(scale, "Read beat", Writer::read_current).disabled(closing || !selected),
    ))
    .gap(8.px())
}
