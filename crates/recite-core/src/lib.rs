//! Shared Recite model types used by the parser, compiler, runtime, CLI, LSP,
//! and adapter tooling.
//!
//! This crate owns the data contracts that must remain consistent across the
//! workspace:
//!
//! - source-level AST values used after parsing and before semantic validation;
//! - stable identifiers, source spans, values, metadata, and structured
//!   diagnostics;
//! - the canonical project schema model and generated manifest loader;
//! - deterministic compiled dialogue tables, fingerprints, and v0 wire
//!   constants.
//!
//! Game code usually reaches these types through `recite-compiler` or
//! `recite-runtime`. Adapter and tooling code may use this crate directly when
//! it needs to inspect schema manifests, compiled assets, diagnostic codes, or
//! stable IDs.
//!
//! The [game-developer guides][guides] and Rust API entry point live in the docs
//! site; this Rustdoc is the library API reference and intentionally does not
//! duplicate the full guide material.
//!
//! [guides]: https://github.com/plethu/recite/tree/main/docs-site/src/content/docs
//!
//! # Example: Load A Schema Manifest
//!
//! ```
//! use recite_core::schema::load_schema_manifest_str;
//!
//! let report = load_schema_manifest_str(
//!     "schema/recite.schema.json",
//!     r#"{
//!       "schema_version": 1,
//!       "speakers": {
//!         "hazel": { "display_name": "Hazel" }
//!       },
//!       "conditions": {
//!         "trust_gte": {
//!           "params": [{ "name": "threshold", "type": "int" }]
//!         }
//!       }
//!     }"#,
//! );
//!
//! assert!(report.diagnostics.is_empty());
//! let schema = report.schema.expect("valid manifest loads");
//! assert!(schema.speakers.contains_key("hazel"));
//! assert!(schema.conditions.contains_key("trust_gte"));
//! ```
//!
//! # Example: Build A Diagnostic
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_core::{
//!     Diagnostic, DiagnosticCategory, DiagnosticCode, SourcePosition, SourceSpan,
//! };
//!
//! let diagnostic = Diagnostic::error(
//!     DiagnosticCode::new_static("RECITE_PARSE001"),
//!     "expected a Recite statement",
//!     SourceSpan::point(
//!         "dialogue/start.recite",
//!         SourcePosition::new(3, 1)?,
//!     ),
//! );
//!
//! assert_eq!(diagnostic.code.category(), DiagnosticCategory::Parse);
//! assert_eq!(diagnostic.span.file, "dialogue/start.recite");
//! # Ok(())
//! # }
//! ```

pub mod ast;
pub mod compiled;
pub mod markup;
pub mod po;
pub mod project;
pub mod schema;

mod diagnostic;
mod diagnostic_argument;
mod diagnostic_code;
mod diagnostic_presentation;
mod diagnostic_presentation_guidance;
mod diagnostic_presentation_record;
mod diagnostic_presentation_wire;
mod diagnostic_record;
mod document_key;
mod error;
mod ids;
mod source_id;
mod source_location;
mod source_recovery;
mod text;
mod toml_spans;
mod value;

pub use diagnostic::{
    Diagnostic, DiagnosticArgumentSpec, DiagnosticArgumentType,
    DiagnosticAuxiliaryPresentationContract, DiagnosticExplanation, DiagnosticPresentationContract,
    DiagnosticPresentationContractRegistryError, DiagnosticSeverity, RelatedSpan,
    auxiliary_contract_for, config_contract_for, contract_for, contracts_for_code,
    default_presentation_id_for_code, explain_diagnostic_code, known_diagnostic_explanations,
    migrated_diagnostic_auxiliary_presentation_contracts,
    migrated_diagnostic_presentation_contracts, presentation_for, suggest_diagnostic_code,
    validate_auxiliary_diagnostic_presentation_contracts,
    validate_diagnostic_presentation_contracts,
    validate_migrated_diagnostic_presentation_contracts,
};
pub use diagnostic_argument::{DiagnosticArgumentValue, DiagnosticFiniteFloat};
pub use diagnostic_code::{DiagnosticCategory, DiagnosticCode};
pub use diagnostic_presentation::{DiagnosticPresentationError, DiagnosticPresentationId};
pub use diagnostic_presentation_guidance::{
    DiagnosticExplanationPresentation, DiagnosticRelatedPresentation,
};
pub use diagnostic_presentation_record::{DiagnosticArguments, DiagnosticPresentation};
pub use diagnostic_record::{DIAGNOSTIC_RECORD_VERSION, DiagnosticRecord, DiagnosticRecordError};
pub use document_key::{DocumentKey, DocumentKeyError};
pub use error::CoreValueError;
pub use ids::{AvailabilityReasonId, BlockId, ChoiceId, EffectId, LineId, LocaleId, SpeakerId};
pub use source_id::{
    SOURCE_ID_ANCHOR_HEX_LEN, SourceAnchor, SourceId, SourceIdKind, is_valid_source_anchor,
    is_valid_source_label,
};
pub use source_location::{SourcePosition, SourceSpan, byte_offset_for_position};
pub use source_recovery::{SourceRecovery, SourceRecoveryClass};
pub use text::{
    PlaceholderSyntaxError, PlaceholderSyntaxKind, PlaceholderValidationError,
    decode_interpolation_text, extract_placeholder_names, extract_placeholder_occurrences,
    validate_translation_placeholders,
};
pub use value::{Metadata, MetadataEntry, ScalarValue, Value};
