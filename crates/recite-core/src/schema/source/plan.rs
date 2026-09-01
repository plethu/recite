use super::toml::{
    SchemaSource, SchemaSourceEdit, SchemaSourceEditError, SchemaSourceStaleDetails,
};
use crate::ContentFingerprint;

/// A non-mutating source edit with optimistic-concurrency preconditions.
///
/// The resulting text is retained so hosts can publish a single replacement
/// without interpreting TOML. Applying it still reparses only at plan
/// construction time; the application step checks both semantic and exact
/// text identities before replacing the source.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SchemaSourceEditPlan {
    expected_file: String,
    expected_source_fingerprint: ContentFingerprint,
    expected_text_fingerprint: ContentFingerprint,
    edit: SchemaSourceEdit,
    updated: SchemaSource,
}

impl SchemaSourceEditPlan {
    pub(super) fn from_source(
        source: &SchemaSource,
        edit: SchemaSourceEdit,
    ) -> Result<Self, SchemaSourceEditError> {
        let mut updated = source.clone();
        super::edit::apply_edit(&mut updated, edit.clone())?;
        Ok(Self {
            expected_file: source.file.clone(),
            expected_source_fingerprint: source.source_fingerprint().clone(),
            expected_text_fingerprint: source.source_text_fingerprint(),
            edit,
            updated,
        })
    }

    /// The typed operation represented by this plan.
    #[must_use]
    pub fn edit(&self) -> &SchemaSourceEdit {
        &self.edit
    }

    /// The source identity against which this plan may be applied.
    #[must_use]
    pub fn expected_source_fingerprint(&self) -> &ContentFingerprint {
        &self.expected_source_fingerprint
    }

    /// The exact text identity against which this plan may be applied.
    #[must_use]
    pub fn expected_text_fingerprint(&self) -> &ContentFingerprint {
        &self.expected_text_fingerprint
    }

    /// The complete source text produced by this plan.
    #[must_use]
    pub fn replacement_text(&self) -> String {
        self.updated.source_text()
    }

    /// Apply this plan if its source and text preconditions still hold.
    pub fn apply(&self, source: &mut SchemaSource) -> Result<(), SchemaSourceEditError> {
        let actual_source_fingerprint = source.source_fingerprint().clone();
        let actual_text_fingerprint = source.source_text_fingerprint();
        if source.file != self.expected_file
            || actual_source_fingerprint != self.expected_source_fingerprint
            || actual_text_fingerprint != self.expected_text_fingerprint
        {
            return Err(SchemaSourceEditError::StaleSource {
                details: Box::new(SchemaSourceStaleDetails {
                    expected_file: self.expected_file.clone(),
                    actual_file: source.file.clone(),
                    expected_source_fingerprint: self.expected_source_fingerprint.clone(),
                    actual_source_fingerprint,
                    expected_text_fingerprint: self.expected_text_fingerprint.clone(),
                    actual_text_fingerprint,
                }),
            });
        }
        *source = self.updated.clone();
        Ok(())
    }
}
