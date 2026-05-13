//! Recite compiler, validator, POT extractor, and compiled asset writer.

mod diagnostics;
mod validation;

pub use validation::{ValidationReport, validate_source_file, validate_source_files};
