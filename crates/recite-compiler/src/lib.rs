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
//! Broader authoring workflow guides live in the
//! [docs site][guides] as they are filled in. This Rustdoc focuses on the
//! library API.
//!
//! [guides]: https://github.com/plethu/recite/tree/main/docs-site/src/content/docs
//!
//! # Example: Compile An In-Memory Scene
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
//! use recite_core::{
//!     compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
//! };
//!
//! let source = concat!(
//!     ":: start default\n",
//!     "> intro_001@8843fd6f53f020a12b31\n",
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
//! assert_eq!(asset.dialogue.lines[0].id.as_str(), "8843fd6f53f020a12b31");
//! assert!(!asset.messagepack.is_empty());
//! # Ok(())
//! # }
//! ```

pub mod authoring;
pub mod compile;
mod diagnostics;
pub mod pot;
pub mod validation;
mod wire;

#[cfg(feature = "bench-support")]
#[doc(hidden)]
pub mod bench_support;
