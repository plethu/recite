use crate::{View, Workbench, WorkbenchError};

/// Applied source and the field draft are separate: rejected prose must survive too.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecoveredDraft {
    source: std::sync::Arc<str>,
    view: View,
    draft: std::sync::Arc<String>,
}

impl RecoveredDraft {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn restore(&self, workbench: &mut Workbench) -> Result<(), WorkbenchError> {
        if workbench.document().source() != self.source.as_ref() {
            return Err(crate::EditError::Stale.into());
        }
        workbench.discard();
        workbench.select(self.view.clone())?;
        workbench.set_draft_snapshot(self.draft.clone());
        Ok(())
    }
}

impl Workbench {
    pub fn recovery(&self) -> RecoveredDraft {
        RecoveredDraft {
            source: self.document().source_snapshot(),
            view: self.view().clone(),
            draft: self.draft_snapshot(),
        }
    }
}
