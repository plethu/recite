use crate::editing::Writer;
use crate::messages::{MsgId, text};
use freya::prelude::*;
use recite_writer_model::RuleExpression;
#[derive(Clone)]
pub(super) struct AddRule {
    pub writer: Writer,
    pub effect: bool,
    pub path: Option<Vec<usize>>,
}
impl PartialEq for AddRule {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.effect == other.effect
            && self.writer.dark == other.writer.dark
    }
}
impl Component for AddRule {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let is_effect = self.effect;
        let path = self.path.clone();
        let options = writer
            .buffers
            .rules
            .read()
            .as_ref()
            .map(|rules| {
                let names: Vec<_> = if is_effect {
                    rules
                        .available_effects
                        .iter()
                        .map(|e| e.function.clone())
                        .collect()
                } else {
                    rules
                        .available_conditions
                        .iter()
                        .filter_map(|e| match e {
                            RuleExpression::Call { function, .. } => Some(function.clone()),
                            _ => None,
                        })
                        .collect()
                };
                names
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| crate::design::PickerOption {
                        annotation: String::new(),
                        value: index.to_string(),
                        title: crate::palette::display_name(&name),
                        detail: name,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let enabled = !options.is_empty()
            && (!is_effect
                || writer
                    .buffers
                    .rules
                    .read()
                    .as_ref()
                    .is_some_and(|r| r.effect_order_editable));
        let caption = if is_effect {
            text(MsgId::WriterRulesAddEffect)
        } else {
            text(MsgId::WriterRulesAddCondition)
        };
        crate::design::SearchPicker {
            id: use_a11y(),
            input_id: use_a11y(),
            name: caption.clone(),
            placeholder: caption.clone(),
            empty_hint: text(MsgId::WriterRulesChooseDeclaration),
            no_matches: text(MsgId::WriterRulesNoDeclarations),
            selected: caption,
            query: use_state(String::new),
            open: use_state(|| false),
            options: std::sync::Arc::new(options),
            choose: EventHandler::new(move |option: crate::design::PickerOption| {
                if let Ok(index) = option.value.parse::<usize>() {
                    super::change(writer, |rules| {
                        if is_effect {
                            if let Some(effect) = rules.available_effects.get(index) {
                                rules.effects.push(effect.clone());
                            }
                        } else if let Some(condition) =
                            rules.available_conditions.get(index).cloned()
                        {
                            if let Some(path) = &path {
                                if let Some(root) = &mut rules.condition
                                    && let Some(
                                        RuleExpression::All(items) | RuleExpression::Any(items),
                                    ) = super::tree::at(root, path)
                                {
                                    items.push(condition);
                                }
                                return;
                            }
                            rules.condition = Some(match rules.condition.take() {
                                None => condition,
                                Some(RuleExpression::All(mut items)) => {
                                    items.push(condition);
                                    RuleExpression::All(items)
                                }
                                Some(existing) => RuleExpression::All(vec![existing, condition]),
                            });
                        }
                    });
                }
            }),
            vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
            enabled,
        }
    }
}
