use super::{Document, EditError};

impl Document {
    pub(crate) fn validate_source_change(&self, source: &str) -> Result<(), EditError> {
        let candidate = Document::in_project(self.key().clone(), source, self.project_context())?;
        self.validate_source_candidate(&candidate)
    }
    pub(crate) fn validate_source_candidate(&self, candidate: &Document) -> Result<(), EditError> {
        let syntax = recite_parser::lower_source_file(candidate.key().as_str(), candidate.source());
        if let Some(diagnostic) = syntax.diagnostics.first() {
            return Err(EditError::InvalidSourceChange(diagnostic.message.clone()));
        }
        let mut remaining = self.diagnostics();
        for diagnostic in candidate.diagnostics() {
            let Some(index) = remaining
                .iter()
                .position(|old| old.code == diagnostic.code && old.message == diagnostic.message)
            else {
                return Err(EditError::InvalidSourceChange(diagnostic.message));
            };
            remaining.swap_remove(index);
        }
        Ok(())
    }
}
