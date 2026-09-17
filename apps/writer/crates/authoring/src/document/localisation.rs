//! Extract the same project that the author sees, including the current overlay.
use crate::Document;
use recite_compiler::{CompileInput, PotExtractionReport, extract_pot, extract_pot_with_schema};

impl Document {
    /// Extract a locale-neutral catalogue without changing source or stable IDs.
    pub fn extract_catalogue(&self) -> PotExtractionReport {
        let mut inputs: Vec<_> = self
            .context
            .documents
            .iter()
            .filter(|saved| saved.key() != self.key())
            .map(|saved| CompileInput::new(saved.key().as_str(), saved.text()))
            .collect();
        inputs.push(CompileInput::new(self.key().as_str(), self.source()));
        match self.schema() {
            Some(schema) => extract_pot_with_schema(inputs, schema),
            None => extract_pot(inputs),
        }
    }
}
