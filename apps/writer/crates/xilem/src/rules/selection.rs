//! Picker receipts name a concrete field and reject changes to its source or structure.
use super::*;
use recite_writer_model::{NavigationStamp, RuleEffect};

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
    origin: NavigationStamp,
    session: SessionIdentity,
    structure: StructureRevision,
    pub choices: Vec<(String, String, Choice)>,
}
impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        writer.rules_editable()
            && self.window == writer.window
            && self.origin.is_current(writer.session.model())
            && writer.rules.as_ref().is_some_and(|reply| {
                reply.identity == self.session && reply.controls.revision == self.structure
            })
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
                    .and_then(|reply| target.get(&reply.projection))
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
        let Some(reply) = &self.rules else { return };
        let rules = &reply.projection;
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
        self.pending_choice = Some(crate::choices::Request::Rules(Request {
            target,
            choices,
            window: self.window,
            origin: self.session.model().navigation_stamp(),
            session: reply.identity.clone(),
            structure: reply.controls.revision,
        }));
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
