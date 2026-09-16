use crate::{View, Workbench, WorkbenchError};

/// Applied source and the field draft are separate: rejected prose must survive too.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecoveredDraft {
    source: String,
    view: View,
    draft: String,
}

impl RecoveredDraft {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn restore(&self, workbench: &mut Workbench) -> Result<(), WorkbenchError> {
        if workbench.document().source() != self.source {
            return Err(crate::EditError::Stale.into());
        }
        workbench.discard();
        workbench.select(self.view.clone())?;
        workbench.set_draft(self.draft.clone());
        Ok(())
    }
}

impl Workbench {
    pub fn recovery(&self) -> RecoveredDraft {
        RecoveredDraft {
            source: self.document().source().to_owned(),
            view: self.view().clone(),
            draft: self.draft().to_owned(),
        }
    }
}
