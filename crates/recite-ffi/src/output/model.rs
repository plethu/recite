use std::fmt;

use rmp_serde::encode;
use serde::Serialize;

pub(crate) const BATCH_FORMAT_VERSION: u16 = 0;

#[derive(Serialize)]
pub(crate) struct FfiOutputBatch {
    pub batch_format_version: u16,
    pub events: Vec<FfiEvent>,
}

/// Serialization failed while building a host-facing output batch.
///
/// This remains private to the FFI implementation. The C ABI flattens it to
/// the stable dialogue-fault status only after the encoder has returned.
#[derive(Debug)]
pub(crate) struct FfiOutputEncodeError {
    pub(super) source: encode::Error,
}

impl fmt::Display for FfiOutputEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to encode FFI output batch: {}",
            self.source
        )
    }
}

impl std::error::Error for FfiOutputEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiEvent {
    Line(FfiLine),
    Prompt {
        line: Option<FfiLine>,
        choices: Vec<FfiChoice>,
    },
    Effect(FfiEffect),
    End {
        deferred_effects: Vec<FfiEffect>,
    },
}

#[derive(Serialize)]
pub(crate) struct FfiLine {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub speaker: Option<String>,
    pub metadata: Vec<FfiMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plural: Option<FfiPlural>,
}

#[derive(Serialize)]
pub(crate) struct FfiPlural {
    pub singular_source_text: String,
    pub plural_source_text: String,
    pub count: i64,
    pub selected_arm: usize,
    pub resolution: FfiPluralResolution,
}

#[derive(Serialize)]
pub(crate) struct FfiPluralResolution {
    pub attempts: Vec<FfiPluralAttempt>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub matched_arm: Option<usize>,
    pub source_fallback_arm: Option<usize>,
    pub outcome: &'static str,
}

#[derive(Serialize)]
pub(crate) struct FfiPluralAttempt {
    pub locale: String,
    pub context: String,
    pub key: String,
    pub selected_arm: Option<usize>,
    pub outcome: &'static str,
}

#[derive(Serialize)]
pub(crate) struct FfiMetadata {
    pub key: String,
    pub value: FfiMetaValue,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiMetaValue {
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
    Array { values: Vec<FfiScalar> },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiScalar {
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

#[derive(Serialize)]
pub(crate) struct FfiChoice {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub metadata: Vec<FfiMetadata>,
    pub echo: FfiEcho,
    pub availability: FfiAvailability,
}

#[derive(Serialize)]
pub(crate) struct FfiEcho {
    pub kind: &'static str,
    pub explicit_line_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct FfiAvailability {
    pub is_available: bool,
    pub primary_reason: Option<FfiAvailabilityReason>,
    pub reason_tree: Option<FfiReasonTree>,
}

#[derive(Serialize)]
pub(crate) struct FfiAvailabilityReason {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub args: Vec<FfiReasonArg>,
}

#[derive(Serialize)]
pub(crate) struct FfiReasonArg {
    pub name: String,
    pub value: FfiReasonValue,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiReasonValue {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiReasonTree {
    All { children: Vec<FfiReasonTree> },
    Any { children: Vec<FfiReasonTree> },
    Reason(FfiAvailabilityReason),
    RequirementSourceText { text: String },
}

#[derive(Serialize)]
pub(crate) struct FfiEffect {
    pub id: String,
    pub mode: &'static str,
    pub function: String,
    pub args: Vec<FfiEffectArg>,
    pub source_file: String,
    pub source_line: u32,
    pub source_col: u32,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiEffectArg {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}
