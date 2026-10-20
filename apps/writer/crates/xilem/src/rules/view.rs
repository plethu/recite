use super::*;
use crate::{
    keyed::keyed,
    presentation::{label, paragraph, text_button},
};
use xilem::{
    InsertNewline, WidgetView,
    masonry::{layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box, text_input},
};
mod structure;
const PAGE: usize = 8;

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ArgumentField(pub bool);
impl xilem::masonry::core::UsesProperty<ArgumentField> for xilem::masonry::widgets::TextInput {}
impl xilem::masonry::core::Property for ArgumentField {
    fn static_default() -> &'static Self {
        static OTHER: ArgumentField = ArgumentField(false);
        &OTHER
    }
}
pub(crate) fn panel(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let disabled = !state.rules_editable();
    let rules = state.rules.as_ref();
    let pages = state.rule_controls.rows.len().div_ceil(PAGE).max(1);
    state.rule_controls.page = state.rule_controls.page.min(pages - 1);
    let page = state.rule_controls.page;
    let rows = state
        .rule_controls
        .rows
        .iter()
        .skip(page * PAGE)
        .take(PAGE)
        .map(|row| match row {
            Row::Condition(path, title) => {
                xilem::core::one_of::Either::A(structure::condition(state, path, title))
            }
            Row::Effect(index, title) => xilem::core::one_of::Either::B(
                xilem::core::one_of::Either::A(structure::effect(state, *index, title)),
            ),
            Row::Argument(target) => xilem::core::one_of::Either::B(
                xilem::core::one_of::Either::B(argument(state, target)),
            ),
        })
        .collect::<Vec<_>>();
    sized_box(flex_col((
        label(scale, "Reply rules").text_size(22. * scale),
        paragraph(scale, rules.map_or_else(String::new, |rules| format!("{} · Destination: {}", rules.text, rules.destination.as_deref().unwrap_or("No destination"))), state.script_width()),
        paragraph(scale, "Conditions control when this reply is available. Effects belong to its destination and can also run when another reply reaches that beat.", state.script_width()),
        rules.filter(|rules| rules.has_other_statements).map(|_| paragraph(scale, "Other statements are preserved. Effect order can change only when the destination contains direct effects alone.", state.script_width())),
        flex_row((
            text_button(scale, "Add condition…", |state: &mut Writer| state.pick_rule_choice(selection::Target::AddCondition(None)))
                .disabled(disabled || rules.is_none_or(|rules| rules.available_conditions.is_empty())),
            text_button(scale, "Add effect…", |state: &mut Writer| state.pick_rule_choice(selection::Target::AddEffect))
                .disabled(disabled || rules.is_none_or(|rules| !rules.effect_order_editable || rules.available_effects.is_empty())),
        )).gap(8.px()),
        rules.filter(|rules| rules.condition.is_none()).map(|_| label(scale, "Available without a condition")),
        rules.filter(|rules| rules.effects.is_empty()).map(|_| label(scale, "No direct effects at this destination")),
        flex_row((
            text_button(scale, "Apply reply", |state: &mut Writer| state.navigate(crate::navigation::Target::Edit(recite_writer_model::EditOperation::Apply)))
                .disabled(disabled || !state.session.model().has_draft()),
            text_button(scale, "Discard reply", Writer::discard_rules).disabled(disabled || !state.session.model().has_draft()),
        )).gap(8.px()),
        text_button(scale, "Edit reply text", Writer::edit_reply_text).disabled(disabled),
        flex_col(rows).cross_axis_alignment(CrossAxisAlignment::Start).gap(16.px()),
        (pages > 1).then(|| flex_col((
            label(scale, format!("Rule fields · {} of {pages}", page + 1)),
            flex_row((text_button(scale, "Previous fields", |state: &mut Writer| state.rule_controls.page = state.rule_controls.page.saturating_sub(1)).disabled(page == 0),
            text_button(scale, "Next fields", |state: &mut Writer| state.rule_controls.page += 1).disabled(page + 1 >= pages),
            )).gap(8.px()),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        (!state.work.busy() && !state.navigation.busy() && rules.is_some_and(|rules| !rules.belongs_to(state.session.model().document()))).then(|| text_button(scale, "Refresh reply controls", |state: &mut Writer| { state.refresh_rules(); })),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())).width(state.script_width().px())
}
fn argument(state: &Writer, target: &ArgumentTarget) -> impl WidgetView<Writer> + use<> {
    let argument = state.rules.as_ref().and_then(|rules| target.get(rules));
    let scale = state.ui_scale();
    let caption = argument.map_or_else(String::new, |arg| {
        let owner = match &target.call {
            CallTarget::Condition(path) => state
                .rules
                .as_ref()
                .and_then(|rules| rules.condition.as_ref())
                .and_then(|root| root.at(path))
                .and_then(|node| match node {
                    RuleExpression::Call { function, .. } => Some(function.as_str()),
                    _ => None,
                })
                .unwrap_or("Condition"),
            CallTarget::Effect(index) => state
                .rules
                .as_ref()
                .and_then(|rules| rules.effects.get(*index))
                .map_or("Effect", |effect| effect.function.as_str()),
        };
        format!("{owner} · {} · {}", arg.label, arg.type_name)
    });
    let target = target.clone();
    let change = target.clone();
    let choose = target.clone();
    let choices = argument.is_some_and(|argument| !argument.choices.is_empty());
    flex_col((
        paragraph(scale, caption.clone(), state.script_width()),
        sized_box(keyed(
            (
                state.session.model().document().key().clone(),
                state.rules.as_ref().map(|rules| rules.passage.clone()),
                state.rule_controls.revision,
                target,
            ),
            text_input(
                argument.map_or_else(String::new, |arg| arg.value.clone()),
                move |state: &mut Writer, value| state.edit_rule_argument(change.clone(), value),
            )
            .text_size(16. * scale)
            .insert_newline(InsertNewline::OnShiftEnter)
            .disabled(!state.rules_editable())
            .prop(TextAreaLabel::new(caption))
            .prop(ArgumentField(true)),
        ))
        .width(state.script_width().px()),
        choices.then(|| {
            text_button(scale, "Choose value…", move |state: &mut Writer| {
                state.pick_rule_choice(selection::Target::Argument(choose.clone()))
            })
            .disabled(!state.rules_editable())
        }),
        argument
            .and_then(|argument| argument.validate().err())
            .map(|error| paragraph(scale, error.to_string(), state.script_width())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
