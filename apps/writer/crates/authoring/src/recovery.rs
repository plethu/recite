use crate::{DraftSnapshot, View, Workbench, WorkbenchError};

/// Applied source and the field draft are separate: rejected prose must survive too.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RecoveredDraft {
    source: std::sync::Arc<str>,
    view: View,
    draft: DraftSnapshot,
    #[serde(default, skip_serializing_if = "crate::SourceValidation::is_freeform")]
    source_validation: crate::SourceValidation,
}

impl RecoveredDraft {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn source_snapshot(&self) -> std::sync::Arc<str> {
        self.source.clone()
    }

    pub const fn view(&self) -> &View {
        &self.view
    }

    pub const fn draft_snapshot(&self) -> &DraftSnapshot {
        &self.draft
    }
    pub const fn source_validation(&self) -> crate::SourceValidation {
        self.source_validation
    }

    pub fn restore(&self, workbench: &mut Workbench) -> Result<(), WorkbenchError> {
        if workbench.document().source() != self.source.as_ref()
            || (!self.source_validation.is_freeform() && self.view != View::Source)
        {
            return Err(crate::EditError::Stale.into());
        }
        workbench.restore_draft(
            self.view.clone(),
            self.draft.clone(),
            self.source_validation,
        )
    }
}

impl Workbench {
    pub fn recovery(&self) -> RecoveredDraft {
        RecoveredDraft {
            source: self.document().source_snapshot(),
            view: self.view().clone(),
            draft: self.draft_snapshot().clone(),
            source_validation: self.source_validation(),
        }
    }
}
