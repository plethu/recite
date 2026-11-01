//! Edits stage atomically through the shared Workbench owner.
use super::*;
impl Writer {
    pub(super) fn stage_rules(&mut self, structural: bool, edit: impl FnOnce(&mut ReplyRules)) {
        if !self.rules_editable() {
            self.status =
                "Finish the current action, then refresh reply controls if needed.".into();
            return;
        }
        let Some(reply) = &mut self.rules else { return };
        let rules = &mut reply.projection;
        self.status = match self
            .session
            .edit(|model| model.stage_reply_rules(rules, edit))
        {
            Ok(()) => {
                if structural {
                    reply.controls.revision.advance();
                    reply.controls.rows = rows::project(rules);
                    reply.controls.expanded = None;
                }
                match rules.validate_draft(self.session.model().document()) {
                    Ok(_) => "Reply draft updated. Apply or Save when ready.".into(),
                    Err(error) => format!("Reply draft kept · {error}"),
                }
            }
            Err(error) => error.to_string(),
        };
    }
    pub(super) fn edit_rule_argument(&mut self, target: ArgumentTarget, value: String) {
        self.stage_rules(false, move |rules| {
            if let Some(argument) = target.get_mut(rules) {
                argument.value = value;
            }
        });
    }
    pub(super) fn remove_condition(&mut self, path: Vec<usize>) {
        self.stage_rules(true, |rules| {
            rules.condition = rules
                .condition
                .take()
                .and_then(|root| root.remove_at(&path));
        });
    }
    pub(super) fn group_condition(&mut self, path: Vec<usize>, grouping: Grouping) {
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
    pub(super) fn remove_effect(&mut self, index: usize) {
        self.stage_rules(true, |rules| {
            if rules.effect_order_editable && index < rules.effects.len() {
                rules.effects.remove(index);
            }
        });
    }
    pub(super) fn move_effect(&mut self, index: usize, forward: bool) {
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
    pub(super) fn effect_mode(&mut self, index: usize, mode: EffectMode) {
        self.stage_rules(false, |rules| {
            if let Some(effect) = rules.effects.get_mut(index)
                && effect.allowed_modes.contains(&mode)
            {
                effect.mode = mode;
            }
        });
    }
}
