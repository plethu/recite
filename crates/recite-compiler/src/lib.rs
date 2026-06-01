//! Recite compiler, validator, POT extractor, and compiled asset writer.
//!
//! This crate turns raw Recite source into deterministic compiled assets for
//! `recite-runtime`. It also exposes validation and gettext POT extraction
//! entry points for CLI, editor, CI, and adapter tooling.
//!
//! `CompileReport` separates recoverable content diagnostics from hard failures:
//! malformed source or invalid schema use returns diagnostics with no asset,
//! while serialization or impossible internal states return `CompileError`.
//! Callers should inspect structured diagnostics instead of parsing rendered
//! messages.
//!
//! For end-to-end authoring workflows, see the
//! [game-developer guides][guides]. This Rustdoc focuses on the library API.
//!
//! [guides]: https://codeberg.org/plethu/recite/src/branch/main/docs-site/src/content/docs
//!
//! # Example: Compile An In-Memory Scene
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
//! use recite_core::{
//!     CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
//! };
//!
//! let source = concat!(
//!     ":: start default\n",
//!     "> intro_001\n",
//!     "  Hello.\n",
//!     "-> END\n",
//! );
//! let options = CompileOptions::new(
//!     CompilerVersion::new("0.0.1")?,
//!     CompiledAssetId::new("example-dialogue")?,
//!     SourceMapId::new("example-source-map")?,
//!     SchemaFingerprint::NoSchema,
//! );
//!
//! let report = compile_inputs(
//!     [CompileInput::new("dialogue/start.recite", source)],
//!     options,
//! )?;
//!
//! assert!(report.diagnostics.is_empty());
//! let asset = report.asset.expect("valid source emits an asset");
//! assert_eq!(asset.dialogue.lines[0].id.as_str(), "intro_001");
//! assert!(!asset.messagepack.is_empty());
//! # Ok(())
//! # }
//! ```

mod compile;
mod diagnostics;
mod pot;
mod validation;
mod wire;

pub use compile::{
    CompileError, CompileInput, CompileOptions, CompileReport, CompiledAssetOutput, compile_inputs,
    compile_inputs_with_schema,
};
pub use pot::{
    PotDocument, PotEntry, PotExtractionReport, PotReference, extract_pot, extract_pot_with_schema,
};
pub use validation::{
    ValidationReport, validate_source_file, validate_source_files,
    validate_source_files_with_schema,
};
