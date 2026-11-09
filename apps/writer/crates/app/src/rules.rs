//! Native rule controls stage edits through the shared authoring owner.
mod editing;
pub mod rows;
pub mod selection;
pub use crate::{Phase, Writer};
use recite_core::ast::EffectMode;
use recite_writer_model::{ReplyRules, RuleArgument, RuleExpression};
pub use rows::Row;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructureRevision(u64);
impl StructureRevision {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallTarget {
    Condition(Vec<usize>),
    Effect(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentTarget {
    pub call: CallTarget,
    index: usize,
}
impl ArgumentTarget {
    pub fn get<'a>(&self, rules: &'a ReplyRules) -> Option<&'a RuleArgument> {
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
pub struct Controls {
    pub revision: StructureRevision,
    pub rows: Vec<Row>,
    pub page: usize,
    pub expanded: Option<CallTarget>,
    refresh_status: Option<String>,
}
/// A projection and all presentation state share one lifetime. The Workbench owns
/// source, draft and history; this session owns only mounted reply controls.
pub struct ReplySession {
    pub projection: ReplyRules,
    document: recite_core::DocumentKey,
    pub identity: SessionIdentity,
    pub controls: Controls,
}
#[derive(Clone, Debug)]
pub struct SessionIdentity(Arc<()>);
impl PartialEq for SessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl ReplySession {
    pub fn new(projection: ReplyRules, document: recite_core::DocumentKey) -> Self {
        let rows = rows::project(&projection);
        Self {
            projection,
            document,
            identity: SessionIdentity(Arc::new(())),
            controls: Controls {
                rows,
                ..Controls::default()
            },
        }
    }
    pub(crate) fn is_open_for(&self, model: &recite_writer_model::Workbench) -> bool {
        model.view() == &recite_writer_model::View::Source
            && self.document == *model.document().key()
    }
    pub(crate) fn projection(&self) -> &ReplyRules {
        &self.projection
    }
    pub(crate) fn take_refresh_status(&mut self) -> Option<String> {
        self.controls.refresh_status.take()
    }
    fn replace(&mut self, projection: ReplyRules) -> ReplyRules {
        let rows = rows::project(&projection);
        if self.controls.rows != rows {
            self.controls.revision.advance();
            self.controls.page = 0;
            self.controls.expanded = None;
        }
        self.controls.rows = rows;
        std::mem::replace(&mut self.projection, projection)
    }
}
#[derive(Clone, Copy)]
pub enum Grouping {
    All,
    Any,
    Not,
}
impl Writer {
    pub(super) fn retire_rules(&mut self) {
        self.rules = None;
        self.pending_choice = None;
        self.location.screen = recite_writer_session::location::Screen::Write;
    }
    pub(super) fn reset_rule_structure(&mut self) {
        if let Some(reply) = &mut self.rules {
            reply.controls.revision.advance();
        }
    }
    pub fn rules_editable(&self) -> bool {
        self.phase == Phase::Writing
            && !self.session.read_only()
            && !self.work.busy()
            && !self.navigation.busy()
            && self.rules.as_ref().is_some_and(|reply| {
                self.session.model().view() == &recite_writer_model::View::Source
                    && reply.projection.belongs_to(self.session.model().document())
            })
    }
    pub(super) fn install_rules(&mut self, rules: &mut Option<ReplyRules>) {
        let document = self.session.model().document().key();
        let old = self.rules.take();
        self.rules = match (old, rules.take()) {
            (Some(mut reply), Some(next))
                if reply.document == *document && reply.projection.passage == next.passage =>
            {
                *rules = Some(reply.replace(next));
                Some(reply)
            }
            (old, next) => {
                *rules = old.map(|reply| reply.projection);
                next.map(|rules| ReplySession::new(rules, document.clone()))
            }
        };
    }
    pub fn refresh_rules(&mut self) {
        if !self.session.model().has_draft()
            && let Some(reply) = &mut self.rules
        {
            reply.controls.refresh_status = Some(self.status.clone());
            if let Err(error) =
                self.begin_navigation(crate::navigation::Target::Location(self.current_location()))
            {
                self.navigation_failed(error);
            }
        }
    }
    pub fn toggle_rule_actions(&mut self, target: CallTarget) {
        let Some(reply) = &mut self.rules else { return };
        reply.controls.expanded = if reply.controls.expanded.as_ref() == Some(&target) {
            None
        } else {
            Some(target)
        };
    }
    pub fn discard_rules(&mut self) {
        if self.rules_editable() {
            self.change(|model| {
                model.discard();
                Ok(())
            });
            self.reset_rule_structure();
            self.refresh_rules();
        }
    }
    pub fn edit_reply_text(&mut self) {
        if let Some(rules) = &self.rules {
            self.navigate(crate::navigation::Target::Location(
                recite_writer_session::location::Location {
                    screen: recite_writer_session::location::Screen::Write,
                    source: false,
                    passage: Some(rules.projection.passage.clone()),
                    view: Some(recite_config::WriterView::Script),
                    ..self.current_location()
                },
            ));
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests;

/// The author-facing name of an effect timing mode.
pub fn effect_mode_name(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "Deferred",
        EffectMode::Immediate => "Immediate",
        EffectMode::Blocking => "Blocking",
    }
}
