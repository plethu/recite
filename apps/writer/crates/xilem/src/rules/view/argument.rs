use super::*;
use crate::WriterLayout;
use crate::keyed::keyed;
use xilem::masonry::core;
use xilem::masonry::widgets;
use xilem::{InsertNewline, masonry::properties::TextAreaLabel, view::text_input};

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ArgumentField(pub bool);
impl core::UsesProperty<ArgumentField> for widgets::TextInput {}
impl core::Property for ArgumentField {
    fn static_default() -> &'static Self {
        static OTHER: ArgumentField = ArgumentField(false);
        &OTHER
    }
}
pub(super) fn field(
    state: &Writer,
    reply: &ReplySession,
    target: &ArgumentTarget,
    owner: &str,
    argument: &RuleArgument,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption = format!("{owner} · {} · {}", argument.label, argument.type_name);
    let change = target.clone();
    let choose = target.clone();
    flex_col((
        paragraph(scale, caption.clone(), state.script_width()),
        sized_box(keyed(
            (
                reply.identity.clone(),
                reply.controls.revision,
                target.clone(),
            ),
            text_input(argument.value.clone(), move |state: &mut Writer, value| {
                state.edit_rule_argument(change.clone(), value)
            })
            .text_size(16. * scale)
            .insert_newline(InsertNewline::OnShiftEnter)
            .disabled(!state.rules_editable())
            .prop(TextAreaLabel::new(caption))
            .prop(ArgumentField(true)),
        ))
        .width(state.script_width().px()),
        (!argument.choices.is_empty()).then(|| {
            text_button(scale, "Choose value…", move |state: &mut Writer| {
                state.pick_rule_choice(selection::Target::Argument(choose.clone()))
            })
            .disabled(!state.rules_editable())
        }),
        argument
            .validate()
            .err()
            .map(|error| paragraph(scale, error.to_string(), state.script_width())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
