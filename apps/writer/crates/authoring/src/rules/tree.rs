//! Structural condition edits shared by native presentation adapters.
use super::{ReplyRules, RuleExpression};

impl RuleExpression {
    pub fn at(&self, path: &[usize]) -> Option<&Self> {
        let Some((first, rest)) = path.split_first() else {
            return Some(self);
        };
        match self {
            Self::All(items) | Self::Any(items) => items.get(*first)?.at(rest),
            Self::Not(inner) | Self::Group(inner) if *first == 0 => inner.at(rest),
            _ => None,
        }
    }
    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Self> {
        let Some((first, rest)) = path.split_first() else {
            return Some(self);
        };
        match self {
            Self::All(items) | Self::Any(items) => items.get_mut(*first)?.at_mut(rest),
            Self::Not(inner) | Self::Group(inner) if *first == 0 => inner.at_mut(rest),
            _ => None,
        }
    }
    pub fn remove_at(mut self, path: &[usize]) -> Option<Self> {
        let (index, rest) = path.split_first()?;
        match &mut self {
            Self::All(items) | Self::Any(items) if *index < items.len() => {
                let child = items.remove(*index);
                if let Some(child) = child.remove_at(rest) {
                    items.insert(*index, child);
                }
                if items.is_empty() {
                    return None;
                }
            }
            Self::Group(inner) | Self::Not(inner) if *index == 0 => {
                **inner = inner.as_ref().clone().remove_at(rest)?;
            }
            _ => {}
        }
        Some(self)
    }
}
impl ReplyRules {
    /// Stale picker indices or paths leave the current condition unchanged.
    pub fn add_condition(&mut self, index: usize, path: Option<&[usize]>) {
        let Some(condition) = self.available_conditions.get(index).cloned() else {
            return;
        };
        if let Some(path) = path {
            if let Some(root) = &mut self.condition
                && let Some(RuleExpression::All(items) | RuleExpression::Any(items)) =
                    root.at_mut(path)
            {
                items.push(condition);
            }
            return;
        }
        self.condition = Some(match self.condition.take() {
            None => condition,
            Some(RuleExpression::All(mut items)) => {
                items.push(condition);
                RuleExpression::All(items)
            }
            Some(existing) => RuleExpression::All(vec![existing, condition]),
        });
    }
    pub fn add_effect(&mut self, index: usize) {
        if self.effect_order_editable
            && let Some(effect) = self.available_effects.get(index)
        {
            self.effects.push(effect.clone());
        }
    }
}
