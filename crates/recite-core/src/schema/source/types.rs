use super::{edit::apply_edit, export::export_json, plan::SchemaSourceEditPlan};
use crate::{
    AvailabilityReasonDefinition, ConditionDefinition, ContentFingerprint, Diagnostic,
    EffectDefinition, ProjectSchema, SchemaFingerprint, canonical_schema_fingerprint,
    canonical_source_fingerprint,
};
use toml_edit::DocumentMut;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaSourceLoadReport {
    pub source: Option<SchemaSource>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct SchemaSource {
    pub(super) file: String,
    pub(super) document: DocumentMut,
    pub(super) source_text: String,
    pub(super) schema: ProjectSchema,
    pub(super) source_fingerprint: ContentFingerprint,
}

impl PartialEq for SchemaSource {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
            && self.source_text == other.source_text
            && self.schema == other.schema
    }
}
impl Eq for SchemaSource {}

impl SchemaSource {
    #[must_use]
    pub fn load_str(file: impl Into<String>, source: &str) -> SchemaSourceLoadReport {
        super::toml::load_schema_source_str(file, source)
    }
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }
    #[must_use]
    pub fn source_text(&self) -> String {
        self.source_text.clone()
    }
    #[must_use]
    pub fn schema(&self) -> &ProjectSchema {
        &self.schema
    }
    #[must_use]
    pub fn source_fingerprint(&self) -> &ContentFingerprint {
        &self.source_fingerprint
    }
    #[must_use]
    pub fn source_text_fingerprint(&self) -> ContentFingerprint {
        canonical_source_fingerprint(&self.source_text)
    }
    #[must_use]
    pub fn schema_fingerprint(&self) -> SchemaFingerprint {
        canonical_schema_fingerprint(&self.schema)
    }
    #[must_use]
    pub fn export_json(&self) -> String {
        export_json(&self.schema)
    }
    pub fn apply_edit(&mut self, edit: SchemaSourceEdit) -> Result<(), SchemaSourceEditError> {
        apply_edit(self, edit)
    }
    pub fn plan_edit(
        &self,
        edit: SchemaSourceEdit,
    ) -> Result<SchemaSourceEditPlan, SchemaSourceEditError> {
        SchemaSourceEditPlan::from_source(self, edit)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum SchemaDeclarationKind {
    Type,
    Registry,
    Speaker,
    Condition,
    AvailabilityReason,
    Effect,
    MetadataDomain,
    Metadata,
    ProjectionQuery,
    PresentationProjector,
    Markup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaSourceEdit {
    SetProducerId(String),
    AddCondition {
        name: String,
        definition: ConditionDefinition,
    },
    AddEffect {
        name: String,
        definition: EffectDefinition,
    },
    AddAvailabilityReason {
        name: String,
        definition: AvailabilityReasonDefinition,
    },
    SetEnumValues {
        name: String,
        values: Vec<String>,
    },
    SetSpeakerDisplayName {
        name: String,
        display_name: Option<String>,
    },
    RemoveDeclaration {
        kind: SchemaDeclarationKind,
        name: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaSourceEditError {
    InvalidArgument(String),
    Diagnostics(Vec<Diagnostic>),
    StaleSource {
        details: Box<SchemaSourceStaleDetails>,
    },
}

/// The identities observed when a source edit plan was applied to stale input.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaSourceStaleDetails {
    pub expected_file: String,
    pub actual_file: String,
    pub expected_source_fingerprint: ContentFingerprint,
    pub actual_source_fingerprint: ContentFingerprint,
    pub expected_text_fingerprint: ContentFingerprint,
    pub actual_text_fingerprint: ContentFingerprint,
}

impl std::fmt::Display for SchemaSourceEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(message) => formatter.write_str(message),
            Self::Diagnostics(diagnostics) => {
                write!(formatter, "{} schema diagnostics", diagnostics.len())
            }
            Self::StaleSource { details } => write!(
                formatter,
                "schema edit plan targets '{}' but current source is '{}' or has changed",
                details.expected_file, details.actual_file
            ),
        }
    }
}
impl std::error::Error for SchemaSourceEditError {}
