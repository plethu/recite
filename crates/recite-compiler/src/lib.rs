//! Recite compiler, validator, POT extractor, and compiled asset writer.

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
