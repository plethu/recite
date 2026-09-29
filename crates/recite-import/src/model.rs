use recite_core::{DiagnosticRecord, SourceSpan};
use serde::{Deserialize, Serialize};

/// Explicit input family; migration never guesses a format from content.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceFamily {
    Json,
    Csv,
    Twee,
    Ink,
    Yarn,
}

/// Location in the original document, not in generated Recite source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Location {
    Document,
    Text {
        span: SourceSpan,
    },
    Json {
        pointer: String,
    },
    Csv {
        row: u64,
        column: usize,
        header: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Provenance {
    pub file: String,
    pub location: Location,
    pub record_key: String,
}

/// Explicit flat-record field mapping. JSON input is an array of objects;
/// CSV input has a header row. Field names are case-sensitive.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FieldMapping {
    pub block: String,
    pub text: String,
    pub id: Option<String>,
    pub speaker: Option<String>,
    pub target: Option<String>,
}

impl FieldMapping {
    #[must_use]
    pub fn new(block: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            block: block.into(),
            text: text.into(),
            id: None,
            speaker: None,
            target: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Action {
    Rejected,
    Skipped,
    ConvertedWithLoss,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ImportItem {
    pub diagnostic: DiagnosticRecord,
    pub provenance: Provenance,
    pub construct: String,
    pub action: Action,
    pub follow_up: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SourceMapping {
    pub provenance: Provenance,
    pub construct: String,
    pub original_id: Option<String>,
    pub generated_id: String,
    pub generated_line: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImportStatus {
    Complete,
    Partial,
    Invalid,
}

/// A single-file migration result. `source` remains inspectable on failure;
/// only Complete/Partial results passed native validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ImportReport {
    pub format_version: u32,
    pub family: SourceFamily,
    pub file: String,
    pub status: ImportStatus,
    pub source: String,
    pub mappings: Vec<SourceMapping>,
    pub items: Vec<ImportItem>,
    pub native_diagnostics: Vec<DiagnosticRecord>,
}

/// Counts for the human summary, grouped by source construct.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct ImportCounts {
    pub generated: usize,
    pub review_items: usize,
}

impl ImportReport {
    #[must_use]
    pub fn counts_by_construct(&self) -> std::collections::BTreeMap<&str, ImportCounts> {
        let mut counts = std::collections::BTreeMap::<&str, ImportCounts>::new();
        for mapping in &self.mappings {
            counts.entry(&mapping.construct).or_default().generated += 1;
        }
        for item in &self.items {
            counts.entry(&item.construct).or_default().review_items += 1;
        }
        if !self.native_diagnostics.is_empty() {
            counts.entry("native_validation").or_default().review_items =
                self.native_diagnostics.len();
        }
        counts
    }
}
