use super::*;

pub(super) fn condition(
    state: &Writer,
    path: &[usize],
    title: &str,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let disabled = !state.rules_editable();
    let node = state
        .rules
        .as_ref()
        .and_then(|rules| rules.condition.as_ref())
        .and_then(|root| root.at(path));
    let group = matches!(node, Some(RuleExpression::All(_) | RuleExpression::Any(_)));
    let path = path.to_vec();
    let all = path.clone();
    let any = path.clone();
    let not = path.clone();
    let remove = path.clone();
    let target = CallTarget::Condition(path.clone());
    let expanded = state.rule_controls.expanded.as_ref() == Some(&target);
    flex_col((
        actions(
            state,
            format!("Condition: {title}"),
            CallTarget::Condition(path.clone()),
            expanded,
        ),
        expanded.then(|| {
            flex_row((
                text_button(
                    scale,
                    if group { "Match all" } else { "Group: all" },
                    move |state: &mut Writer| state.group_condition(all.clone(), Grouping::All),
                )
                .disabled(disabled || matches!(node, Some(RuleExpression::All(_)))),
                text_button(
                    scale,
                    if group { "Match any" } else { "Group: any" },
                    move |state: &mut Writer| state.group_condition(any.clone(), Grouping::Any),
                )
                .disabled(disabled || matches!(node, Some(RuleExpression::Any(_)))),
                text_button(
                    scale,
                    if matches!(node, Some(RuleExpression::Not(_))) {
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
        }),
        expanded.then(|| {
            flex_col((
                group.then(|| {
                    text_button(scale, "Add within group…", move |state: &mut Writer| {
                        state.pick_rule_choice(selection::Target::AddCondition(Some(path.clone())))
                    })
                    .disabled(disabled)
                }),
                text_button(
                    scale,
                    format!("Remove condition {title}"),
                    move |state: &mut Writer| state.remove_condition(remove.clone()),
                )
                .disabled(disabled),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
pub(super) fn effect(state: &Writer, index: usize, title: &str) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let effect = state
        .rules
        .as_ref()
        .and_then(|rules| rules.effects.get(index));
    let disabled = !state.rules_editable();
    let reorder = !disabled
        && state
            .rules
            .as_ref()
            .is_some_and(|rules| rules.effect_order_editable);
    let count = state.rules.as_ref().map_or(0, |rules| rules.effects.len());
    let expanded = state.rule_controls.expanded.as_ref() == Some(&CallTarget::Effect(index));
    let mode = effect.map_or("", |effect| mode_name(effect.mode));
    flex_col((
        actions(
            state,
            format!("Effect {}: {title} · {mode}", index + 1),
            CallTarget::Effect(index),
            expanded,
        ),
        expanded.then(|| {
            flex_col((
                text_button(scale, "Move effect up", move |state: &mut Writer| {
                    state.move_effect(index, false)
                })
                .disabled(!reorder || index == 0),
                text_button(scale, "Move effect down", move |state: &mut Writer| {
                    state.move_effect(index, true)
                })
                .disabled(!reorder || index + 1 >= count),
                text_button(scale, "Remove effect", move |state: &mut Writer| {
                    state.remove_effect(index)
                })
                .disabled(!reorder),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
        expanded.then(|| {
            flex_col(
                effect
                    .into_iter()
                    .flat_map(|effect| {
                        effect.allowed_modes.iter().map(move |mode| {
                            let mode = *mode;
                            let caption = mode_name(mode);
                            text_button(
                                scale,
                                format!(
                                    "{}Delivery: {caption}",
                                    if mode == effect.mode {
                                        "Selected · "
                                    } else {
                                        ""
                                    }
                                ),
                                move |state: &mut Writer| state.effect_mode(index, mode),
                            )
                            .disabled(disabled || mode == effect.mode)
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(4.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}

fn mode_name(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "Deferred",
        EffectMode::Immediate => "Immediate",
        EffectMode::Blocking => "Blocking",
    }
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
        .prop(xilem::masonry::properties::LineBreaking::WordWrap),
        move |state: &mut Writer| state.toggle_rule_actions(target.clone()),
    ))
    .width(state.script_width().px())
}
