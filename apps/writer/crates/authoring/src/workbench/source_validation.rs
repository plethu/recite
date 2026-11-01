//! Structured controls retain their validation contract while detached or recovered.
use super::{View, Workbench, WorkbenchError};
use crate::{DraftSnapshot, EditError, ReplyRules};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SourceValidation {
    /// A source editor may retain malformed accepted text for diagnostics and repair.
    #[default]
    Freeform,
    /// Structured edits and imported repairs cannot introduce syntax or project errors.
    NoNewErrors,
}
impl SourceValidation {
    pub const fn is_freeform(&self) -> bool {
        matches!(self, Self::Freeform)
    }
}
impl Workbench {
    pub const fn source_validation(&self) -> SourceValidation {
        self.source_validation
    }
    pub fn open_reply_rules(&mut self, passage: &str) -> Result<ReplyRules, WorkbenchError> {
        self.require_applied()?;
        let rules = self.document.reply_rules(passage)?;
        self.select(View::Source)?;
        self.source_validation = SourceValidation::NoNewErrors;
        Ok(rules)
    }
    /// Stage an explicitly imported repair; Apply still validates before acceptance.
    pub fn stage_repaired_source(&mut self, source: DraftSnapshot) -> Result<(), WorkbenchError> {
        if self.view != View::Source {
            return Err(EditError::SourceRequired("open the recovered source draft first").into());
        }
        self.set_draft_snapshot(source);
        self.source_validation = SourceValidation::NoNewErrors;
        Ok(())
    }
}
