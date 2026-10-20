//! Native rule controls stage edits through the shared authoring owner.
mod rows;
pub(crate) mod selection;
pub(super) mod view;
use crate::{Phase, Writer};
use recite_core::ast::EffectMode;
use recite_writer_model::{ReplyRules, RuleArgument, RuleExpression};
use rows::Row;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StructureRevision(u64);
impl StructureRevision {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CallTarget {
    Condition(Vec<usize>),
    Effect(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArgumentTarget {
    call: CallTarget,
    index: usize,
}
impl ArgumentTarget {
    fn get<'a>(&self, rules: &'a ReplyRules) -> Option<&'a RuleArgument> {
        match &self.call {
            CallTarget::Condition(path) => match rules.condition.as_ref()?.at(path)? {
                RuleExpression::Call { arguments, .. } => arguments.get(self.index),
                _ => None,
            },
            CallTarget::Effect(index) => rules.effects.get(*index)?.arguments.get(self.index),
        }
    }
    fn get_mut<'a>(&self, rules: &'a mut ReplyRules) -> Option<&'a mut RuleArgument> {
        match &self.call {
            CallTarget::Condition(path) => match rules.condition.as_mut()?.at_mut(path)? {
                RuleExpression::Call { arguments, .. } => arguments.get_mut(self.index),
                _ => None,
            },
            CallTarget::Effect(index) => {
                rules.effects.get_mut(*index)?.arguments.get_mut(self.index)
            }
        }
    }
}
#[derive(Default)]
pub(super) struct Controls {
    revision: StructureRevision,
    rows: Vec<Row>,
    page: usize,
    expanded: Option<CallTarget>,
    pub pending_choice: Option<selection::Request>,
    pub refresh_status: Option<String>,
}
#[derive(Clone, Copy)]
enum Grouping {
    All,
    Any,
    Not,
}
impl Writer {
    pub(super) fn reset_rule_structure(&mut self) {
        self.rule_controls.revision.advance();
    }
    pub(crate) fn rules_editable(&self) -> bool {
        self.phase == Phase::Writing
            && !self.work.busy()
            && !self.navigation.busy()
            && self.rules.as_ref().is_some_and(|rules| {
                self.session.model().view() == &recite_writer_model::View::Source
                    && rules.belongs_to(self.session.model().document())
            })
    }
    pub(super) fn install_rules(&mut self, rules: &mut Option<ReplyRules>) {
        if !self
            .rules
            .as_ref()
            .zip(rules.as_ref())
            .is_some_and(|(old, next)| {
                old.passage == next.passage && rows::same_structure(old, next)
            })
        {
            self.rule_controls.revision.advance();
            self.rule_controls.page = 0;
        }
        std::mem::swap(&mut self.rules, rules);
        self.rule_controls.rows = self.rules.as_ref().map(rows::project).unwrap_or_default();
    }
    fn stage_rules(&mut self, structural: bool, edit: impl FnOnce(&mut ReplyRules)) {
        if !self.rules_editable() {
            self.status =
                "Finish the current action, then refresh reply controls if needed.".into();
            return;
        }
        let Some(rules) = &mut self.rules else { return };
        self.status = match self
            .session
            .edit(|model| model.stage_reply_rules(rules, edit))
        {
            Ok(()) => {
                if structural {
                    self.rule_controls.revision.advance();
                    self.rule_controls.rows = rows::project(rules);
                    self.rule_controls.expanded = None;
                }
                match rules.validate_draft(self.session.model().document()) {
                    Ok(_) => "Reply draft updated. Apply or Save when ready.".into(),
                    Err(error) => format!("Reply draft kept · {error}"),
                }
            }
            Err(error) => error.to_string(),
        };
    }
    fn edit_rule_argument(&mut self, target: ArgumentTarget, value: String) {
        self.stage_rules(false, move |rules| {
            if let Some(argument) = target.get_mut(rules) {
                argument.value = value;
            }
        });
    }
    fn remove_condition(&mut self, path: Vec<usize>) {
        self.stage_rules(true, |rules| {
            rules.condition = rules
                .condition
                .take()
                .and_then(|root| root.remove_at(&path));
        });
    }
    fn group_condition(&mut self, path: Vec<usize>, grouping: Grouping) {
        self.stage_rules(true, |rules| {
            if let Some(node) = rules.condition.as_mut().and_then(|root| root.at_mut(&path)) {
                let old = node.clone();
                *node = match grouping {
                    Grouping::All => match old {
                        RuleExpression::Any(items) => RuleExpression::All(items),
                        old => RuleExpression::All(vec![old]),
                    },
                    Grouping::Any => match old {
                        RuleExpression::All(items) => RuleExpression::Any(items),
                        old => RuleExpression::Any(vec![old]),
                    },
                    Grouping::Not => match old {
                        RuleExpression::Not(inner) => *inner,
                        old => RuleExpression::Not(Box::new(old)),
                    },
                };
            }
        });
    }
    fn remove_effect(&mut self, index: usize) {
        self.stage_rules(true, |rules| {
            if rules.effect_order_editable && index < rules.effects.len() {
                rules.effects.remove(index);
            }
        });
    }
    fn move_effect(&mut self, index: usize, forward: bool) {
        self.stage_rules(true, |rules| {
            let next = if forward {
                index.checked_add(1)
            } else {
                index.checked_sub(1)
            };
            if rules.effect_order_editable
                && index < rules.effects.len()
                && let Some(next) = next.filter(|next| *next < rules.effects.len())
            {
                rules.effects.swap(index, next);
            }
        });
    }
    fn effect_mode(&mut self, index: usize, mode: EffectMode) {
        self.stage_rules(false, |rules| {
            if let Some(effect) = rules.effects.get_mut(index)
                && effect.allowed_modes.contains(&mode)
            {
                effect.mode = mode;
            }
        });
    }
    pub(super) fn refresh_rules(&mut self) {
        if self.rules.is_some() && !self.session.model().has_draft() {
            self.rule_controls.refresh_status = Some(self.status.clone());
            if let Err(error) =
                self.begin_navigation(crate::navigation::Target::Location(self.current_location()))
            {
                self.navigation_failed(error);
            }
        }
    }
    fn toggle_rule_actions(&mut self, target: CallTarget) {
        self.rule_controls.expanded = if self.rule_controls.expanded.as_ref() == Some(&target) {
            None
        } else {
            Some(target)
        };
    }
    fn discard_rules(&mut self) {
        if self.rules_editable() {
            self.change(|model| {
                model.discard();
                Ok(())
            });
            self.rule_controls.revision.advance();
            self.refresh_rules();
        }
    }
    fn edit_reply_text(&mut self) {
        if let Some(rules) = &self.rules {
            self.navigate(crate::navigation::Target::Location(
                recite_writer_session::location::Location {
                    screen: recite_writer_session::location::Screen::Write,
                    source: false,
                    passage: Some(rules.passage.clone()),
                    view: Some(recite_config::WriterView::Script),
                    ..self.current_location()
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests;
