//! Metadata reloads use output identity without rebasing a standalone source buffer.
use super::super::check_owner;
use super::*;
use crate::{declarations::producer::Generated, project::read_regular};
impl DeclarationWork {
    pub(super) fn reload_generated(&self) -> Result<PreparedDeclaration, FileError> {
        let text = read_regular(&self.output)?;
        let loaded =
            recite_core::schema::load_schema_manifest_str(self.output.to_string_lossy(), &text);
        let schema = loaded
            .schema
            .ok_or(FileError::Validation(loaded.diagnostics))?;
        check_owner(self.schema_output.last_valid_schema(), &schema)?;
        Ok(PreparedDeclaration::Output(Arc::new(SchemaOutput::Ready(
            Generated {
                text,
                schema: Arc::new(schema),
            },
        ))))
    }
    pub(super) fn reload_registration(&self) -> PreparedDeclaration {
        PreparedDeclaration::Registration(Session::registration(
            &self.root,
            self.schema_output.last_valid_schema(),
        ))
    }
}

impl Session {
    pub fn reload_generated(&mut self) -> Result<(), FileError> {
        self.declaration_blocking(DeclarationAction::ReloadGenerated)
            .map(drop)
    }
    pub fn reload_registration(&mut self) {
        if !self.busy() && !self.saving {
            let _ = self.declaration_blocking(DeclarationAction::ReloadRegistration);
        }
    }
}

#[cfg(test)]
mod tests;
