//! Picker receipts name a concrete field and reject changes to its source or structure.
use super::*;
use recite_writer_model::{AnalysisStamp, RecoveredDraft, RuleEffect};

#[derive(Clone)]
pub(crate) enum Choice {
    Condition(RuleExpression),
    Effect(Box<RuleEffect>),
    Argument(String),
}
#[derive(Clone)]
pub(crate) enum Target {
    AddCondition(Option<Vec<usize>>),
    AddEffect,
    Argument(ArgumentTarget),
}
pub(crate) struct Request {
    target: Target,
    window: xilem::WindowId,
    stamp: AnalysisStamp,
    draft: RecoveredDraft,
    passage: String,
    structure: StructureRevision,
    pub choices: Vec<(String, String, Choice)>,
}
impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        writer.rules_editable()
            && self.window == writer.window
            && self.stamp.is_current(writer.session.model().document())
            && self.draft == writer.session.model().recovery()
            && writer
                .rules
                .as_ref()
                .is_some_and(|rules| rules.passage == self.passage)
            && self.structure == writer.rule_controls.revision
    }
    pub fn apply(&self, writer: &mut Writer, choice: Choice) -> bool {
        if !self.is_current(writer) {
            writer.status = "This reply choice changed. Reopen the choice to continue.".into();
            return false;
        }
        match (&self.target, choice) {
            (Target::Argument(target), Choice::Argument(value)) => {
                if !writer
                    .rules
                    .as_ref()
                    .and_then(|rules| target.get(rules))
                    .is_some_and(|argument| argument.choices.contains(&value))
                {
                    return false;
                }
                writer.edit_rule_argument(target.clone(), value);
            }
            (Target::AddCondition(path), Choice::Condition(condition)) => {
                writer.stage_rules(true, |rules| {
                    if let Some(index) = rules
                        .available_conditions
                        .iter()
                        .position(|item| *item == condition)
                    {
                        rules.add_condition(index, path.as_deref());
                    }
                });
            }
            (Target::AddEffect, Choice::Effect(effect)) => {
                writer.stage_rules(true, |rules| {
                    if let Some(index) = rules
                        .available_effects
                        .iter()
                        .position(|item| item == effect.as_ref())
                    {
                        rules.add_effect(index);
                    }
                });
            }
            _ => return false,
        }
        true
    }
}
impl Writer {
    pub(crate) fn pick_rule_choice(&mut self, target: Target) {
        if !self.rules_editable() {
            return;
        }
        let Some(rules) = &self.rules else { return };
        let choices = match &target {
            Target::AddCondition(_) => rules
                .available_conditions
                .iter()
                .filter_map(|item| {
                    let RuleExpression::Call {
                        function,
                        arguments,
                    } = item
                    else {
                        return None;
                    };
                    Some((
                        function.clone(),
                        signature(arguments),
                        Choice::Condition(item.clone()),
                    ))
                })
                .collect(),
            Target::AddEffect => rules
                .available_effects
                .iter()
                .map(|item| {
                    (
                        item.function.clone(),
                        signature(&item.arguments),
                        Choice::Effect(Box::new(item.clone())),
                    )
                })
                .collect(),
            Target::Argument(target) => target
                .get(rules)
                .into_iter()
                .flat_map(|argument| {
                    argument.choices.iter().map(|value| {
                        (
                            value.clone(),
                            argument.type_name.clone(),
                            Choice::Argument(value.clone()),
                        )
                    })
                })
                .collect(),
        };
        self.rule_controls.pending_choice = Some(Request {
            target,
            choices,
            window: self.window,
            stamp: self.session.model().document().analysis_stamp(),
            draft: self.session.model().recovery(),
            passage: rules.passage.clone(),
            structure: self.rule_controls.revision,
        });
    }
}
fn signature(arguments: &[RuleArgument]) -> String {
    if arguments.is_empty() {
        return "No arguments".into();
    }
    arguments
        .iter()
        .map(|arg| format!("{} · {}", arg.label, arg.type_name))
        .collect::<Vec<_>>()
        .join(", ")
}
