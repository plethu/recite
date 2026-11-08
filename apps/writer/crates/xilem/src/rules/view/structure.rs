use super::*;
use crate::WriterLayout;
use recite_writer_model::RuleEffect;
use xilem::masonry::properties;

pub(super) fn condition(
    state: &Writer,
    reply: &ReplySession,
    path: &[usize],
    title: &str,
    node: &RuleExpression,
) -> impl WidgetView<Writer> + use<> {
    let target = CallTarget::Condition(path.to_vec());
    let expanded = reply.controls.expanded.as_ref() == Some(&target);
    flex_col((
        actions(state, format!("Condition: {title}"), target, expanded),
        expanded.then(|| grouping(state, path, node)),
        expanded.then(|| condition_actions(state, path, title, node)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
fn grouping(
    state: &Writer,
    path: &[usize],
    node: &RuleExpression,
) -> impl WidgetView<Writer> + use<> {
    let disabled = !state.rules_editable();
    let group = matches!(node, RuleExpression::All(_) | RuleExpression::Any(_));
    let all = path.to_vec();
    let any = path.to_vec();
    let not = path.to_vec();
    flex_row((
        text_button(
            state.ui_scale(),
            if group { "Match all" } else { "Group: all" },
            move |state: &mut Writer| state.group_condition(all.clone(), Grouping::All),
        )
        .disabled(disabled || matches!(node, RuleExpression::All(_))),
        text_button(
            state.ui_scale(),
            if group { "Match any" } else { "Group: any" },
            move |state: &mut Writer| state.group_condition(any.clone(), Grouping::Any),
        )
        .disabled(disabled || matches!(node, RuleExpression::Any(_))),
        text_button(
            state.ui_scale(),
            if matches!(node, RuleExpression::Not(_)) {
                "Remove negation"
            } else {
                "Negate"
            },
            move |state: &mut Writer| state.group_condition(not.clone(), Grouping::Not),
        )
        .disabled(disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
fn condition_actions(
    state: &Writer,
    path: &[usize],
    title: &str,
    node: &RuleExpression,
) -> impl WidgetView<Writer> + use<> {
    let add = path.to_vec();
    let remove = path.to_vec();
    flex_col((
        matches!(node, RuleExpression::All(_) | RuleExpression::Any(_)).then(|| {
            text_button(
                state.ui_scale(),
                "Add within group…",
                move |state: &mut Writer| {
                    state.pick_rule_choice(selection::Target::AddCondition(Some(add.clone())))
                },
            )
            .disabled(!state.rules_editable())
        }),
        text_button(
            state.ui_scale(),
            format!("Remove condition {title}"),
            move |state: &mut Writer| state.remove_condition(remove.clone()),
        )
        .disabled(!state.rules_editable()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
pub(super) fn effect(
    state: &Writer,
    reply: &ReplySession,
    index: usize,
    effect: &RuleEffect,
) -> impl WidgetView<Writer> + use<> {
    let target = CallTarget::Effect(index);
    let expanded = reply.controls.expanded.as_ref() == Some(&target);
    flex_col((
        actions(
            state,
            format!(
                "Effect {}: {} · {}",
                index + 1,
                effect.function,
                recite_writer_app::rules::effect_mode_name(effect.mode)
            ),
            target,
            expanded,
        ),
        expanded.then(|| effect_order(state, &reply.projection, index)),
        expanded.then(|| delivery_modes(state, index, effect)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
fn effect_order(
    state: &Writer,
    rules: &ReplyRules,
    index: usize,
) -> impl WidgetView<Writer> + use<> {
    let disabled = !state.rules_editable() || !rules.effect_order_editable;
    flex_col((
        text_button(
            state.ui_scale(),
            "Move effect up",
            move |state: &mut Writer| state.move_effect(index, false),
        )
        .disabled(disabled || index == 0),
        text_button(
            state.ui_scale(),
            "Move effect down",
            move |state: &mut Writer| state.move_effect(index, true),
        )
        .disabled(disabled || index + 1 >= rules.effects.len()),
        text_button(
            state.ui_scale(),
            "Remove effect",
            move |state: &mut Writer| state.remove_effect(index),
        )
        .disabled(disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
fn delivery_modes(
    state: &Writer,
    index: usize,
    effect: &RuleEffect,
) -> impl WidgetView<Writer> + use<> {
    let modes = effect
        .allowed_modes
        .iter()
        .map(|mode| {
            let mode = *mode;
            let selected = mode == effect.mode;
            text_button(
                state.ui_scale(),
                format!(
                    "{}Delivery: {}",
                    if selected { "Selected · " } else { "" },
                    recite_writer_app::rules::effect_mode_name(mode)
                ),
                move |state: &mut Writer| state.effect_mode(index, mode),
            )
            .disabled(!state.rules_editable() || selected)
        })
        .collect::<Vec<_>>();
    flex_col(modes)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(4.px())
}
fn actions(
    state: &Writer,
    title: String,
    target: CallTarget,
    expanded: bool,
) -> impl WidgetView<Writer> + use<> {
    sized_box(xilem::view::button(
        label(
            state.ui_scale(),
            format!(
                "{title} · {} actions",
                if expanded { "Hide" } else { "Show" }
            ),
        )
        .prop(properties::LineBreaking::WordWrap),
        move |state: &mut Writer| state.toggle_rule_actions(target.clone()),
    ))
    .width(state.script_width().px())
}
