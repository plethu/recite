//! Bounded migration to ordinary Recite source, with inspectable provenance.
//!
//! This crate performs no filesystem writes and executes no source-language
//! scripts. Callers own inspection, author review and publication of results.

mod builder;
mod diagnostics;
mod model;
mod records;
mod text;

pub use model::{
    Action, FieldMapping, ImportCounts, ImportItem, ImportReport, ImportStatus, Location,
    Provenance, SourceFamily, SourceMapping,
};

use recite_core::schema::ProjectSchema;

/// Inputs shared by library and CLI callers.
pub struct ImportRequest<'a> {
    pub family: SourceFamily,
    pub file: &'a str,
    pub source: &'a str,
    pub mapping: Option<&'a FieldMapping>,
    pub schema: Option<&'a ProjectSchema>,
}

/// Internal/native contract failures, distinct from reported bad input.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ImportError {
    #[error(transparent)]
    Core(#[from] recite_core::CoreValueError),
    #[error(transparent)]
    Presentation(#[from] recite_core::DiagnosticPresentationError),
    #[error(transparent)]
    Record(#[from] recite_core::DiagnosticRecordError),
    #[error("missing diagnostic contract: {0}")]
    MissingContract(&'static str),
    #[error("source position exceeds the supported range")]
    PositionOverflow,
    #[error(transparent)]
    Compile(#[from] recite_compiler::compile::CompileError),
    #[error(transparent)]
    CompiledValue(#[from] recite_core::compiled::CompiledValueError),
}

/// Inspect a bounded input and validate its generated source using the native
/// parser and compiler validators. Invalid input is represented in the report.
pub fn import(request: ImportRequest<'_>) -> Result<ImportReport, ImportError> {
    let mut builder = builder::Builder::new(request.family, request.file);
    match request.family {
        SourceFamily::Json | SourceFamily::Csv => records::read(&request, &mut builder)?,
        SourceFamily::Twee | SourceFamily::Ink | SourceFamily::Yarn => {
            text::read(&request, &mut builder)?;
        }
    }
    builder.finish(request.schema)
}
