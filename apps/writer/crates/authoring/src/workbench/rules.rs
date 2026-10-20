//! Rule controls stage source atomically, retaining invalid arguments for recovery and feedback.
use super::{SourceValidation, View, Workbench, WorkbenchError};
use crate::{DraftSnapshot, EditError, ReplyRules};

impl Workbench {
    pub fn stage_reply_rules(
        &mut self,
        rules: &mut ReplyRules,
        edit: impl FnOnce(&mut ReplyRules),
    ) -> Result<(), WorkbenchError> {
        if self.view != View::Source || !rules.belongs_to(self.document()) {
            return Err(EditError::Stale.into());
        }
        let previous = DraftSnapshot::from(rules.source()?);
        if self.draft_snapshot() != &previous {
            return Err(EditError::Stale.into());
        }
        let mut next = rules.clone();
        edit(&mut next);
        let source = next.source()?;
        self.set_draft(source);
        self.source_validation = SourceValidation::NoNewErrors;
        *rules = next;
        Ok(())
    }
}
