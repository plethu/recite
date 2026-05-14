use recite_core::{
    CompiledAssetId, CompiledDialogue, CompilerVersion, Diagnostic, SchemaFingerprint, SourceMapId,
};
use recite_parser::parse;

use super::CompileError;
use super::builder::build_dialogue;
use super::lowered::LoweredInput;
use crate::validation::{project::sort_diagnostics_by_source, validate_source_files};
use crate::wire::{serialize_inspection_json, serialize_messagepack};

/// Raw source input for one file in a compiler invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileInput {
    pub path: String,
    pub source: String,
}

impl CompileInput {
    #[must_use]
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
        }
    }
}

/// Options that become part of the v0 compiled asset header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileOptions {
    pub compiler_version: CompilerVersion,
    pub asset_id: CompiledAssetId,
    pub source_map_id: SourceMapId,
    pub schema_fingerprint: SchemaFingerprint,
}

impl CompileOptions {
    #[must_use]
    pub fn new(
        compiler_version: CompilerVersion,
        asset_id: CompiledAssetId,
        source_map_id: SourceMapId,
        schema_fingerprint: SchemaFingerprint,
    ) -> Self {
        Self {
            compiler_version,
            asset_id,
            source_map_id,
            schema_fingerprint,
        }
    }
}

/// Result of compiling raw Recite inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileReport {
    pub diagnostics: Vec<Diagnostic>,
    pub asset: Option<CompiledAssetOutput>,
}

impl CompileReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty() && self.asset.is_some()
    }
}

/// Runtime-facing compiled asset plus deterministic inspection output.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledAssetOutput {
    pub dialogue: CompiledDialogue,
    pub messagepack: Vec<u8>,
    pub inspection_json: String,
}

/// Compile raw Recite source inputs into a deterministic v0 compiled asset.
pub fn compile_inputs(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
) -> Result<CompileReport, CompileError> {
    let mut lowered_inputs = Vec::new();
    let mut diagnostics = Vec::new();

    for (input_index, input) in inputs.into_iter().enumerate() {
        let parse = parse(&input.path, &input.source);
        let lowered = parse.lower_source_file();
        diagnostics.extend(lowered.diagnostics);
        lowered_inputs.push(LoweredInput {
            input_index,
            source: input.source,
            source_file: lowered.source_file,
        });
    }

    sort_diagnostics_by_source(&mut diagnostics);
    if !diagnostics.is_empty() {
        return Ok(CompileReport {
            diagnostics,
            asset: None,
        });
    }

    let source_files = lowered_inputs
        .iter()
        .map(|input| input.source_file.clone())
        .collect::<Vec<_>>();
    let validation = validate_source_files(&source_files);
    if !validation.is_ok() {
        return Ok(CompileReport {
            diagnostics: validation.diagnostics,
            asset: None,
        });
    }

    lowered_inputs.sort_by(|left, right| {
        left.source_file
            .path
            .cmp(&right.source_file.path)
            .then(left.input_index.cmp(&right.input_index))
    });

    let dialogue = build_dialogue(&lowered_inputs, options)?;
    let messagepack = serialize_messagepack(&dialogue)?;
    let inspection_json = serialize_inspection_json(&dialogue)?;

    Ok(CompileReport {
        diagnostics: Vec::new(),
        asset: Some(CompiledAssetOutput {
            dialogue,
            messagepack,
            inspection_json,
        }),
    })
}
