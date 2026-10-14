use recite_core::{
    Diagnostic,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
    schema::ProjectSchema,
};

use super::CompileError;
use super::pipeline::{Input, compile_inputs_with_output};
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

/// Result of compiling only the runtime-facing canonical MessagePack bytes.
///
/// Content diagnostics prevent output, just as in [`CompileReport`]. This
/// operation performs complete compilation and validation without retaining
/// the compiled model or producing inspection JSON.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct MessagepackCompileReport {
    pub diagnostics: Vec<Diagnostic>,
    pub messagepack: Option<Vec<u8>>,
}

impl MessagepackCompileReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty() && self.messagepack.is_some()
    }
}

/// Compiled model after complete source and optional schema validation.
///
/// No asset bytes, inspection JSON or canonical fingerprint are produced.
/// Encode the model with the core codec to validate its compiled payload and
/// produce canonical runtime bytes. Content diagnostics prevent output.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct ModelCompileReport {
    pub diagnostics: Vec<Diagnostic>,
    pub dialogue: Option<CompiledDialogue>,
}

impl ModelCompileReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty() && self.dialogue.is_some()
    }
}

/// Compile a model without encoding, using complete project validation.
///
/// Sources, diagnostics and table order match [`compile_inputs`] and
/// [`compile_inputs_with_schema`]. Canonical compiled-payload validation and
/// fingerprinting remain the encoder's responsibility.
pub fn compile_inputs_to_model(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
) -> Result<ModelCompileReport, CompileError> {
    let report =
        compile_inputs_with_output(inputs.into_iter().map(Input::Raw), options, schema, Ok)?;
    Ok(ModelCompileReport {
        diagnostics: report.diagnostics,
        dialogue: report.output,
    })
}

/// Compile immutable captured build inputs without copying intermediate source strings.
///
/// Reuses fingerprints owned by the request while retaining complete source and
/// schema validation. Source ordering and diagnostics match [`compile_inputs_to_model`].
pub fn compile_build_request_to_model(
    request: &crate::authoring::BuildRequest,
    options: CompileOptions,
) -> Result<ModelCompileReport, CompileError> {
    let schema = request
        .inputs()
        .iter()
        .find_map(|input| input.schema_model());
    let inputs = request
        .inputs()
        .iter()
        .filter(|input| input.kind() == &crate::authoring::BuildInputKind::Source)
        .map(Input::Captured);
    let report = compile_inputs_with_output(inputs, options, schema, Ok)?;
    Ok(ModelCompileReport {
        diagnostics: report.diagnostics,
        dialogue: report.output,
    })
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
    compile_inputs_with_optional_schema(inputs, options, None)
}

/// Compile raw Recite source inputs against a loaded project schema.
pub fn compile_inputs_with_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: &ProjectSchema,
) -> Result<CompileReport, CompileError> {
    compile_inputs_with_optional_schema(inputs, options, Some(schema))
}

/// Compile raw sources into canonical MessagePack without inspection output.
///
/// Uses the same complete validation and deterministic compilation as
/// [`compile_inputs`]. Content failures return diagnostics with no bytes;
/// non-content failures return [`CompileError`].
pub fn compile_inputs_to_messagepack(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
) -> Result<MessagepackCompileReport, CompileError> {
    compile_inputs_to_messagepack_with_optional_schema(inputs, options, None)
}

/// Compile canonical MessagePack against a loaded schema without inspection output.
///
/// Validation, diagnostics and asset bytes match [`compile_inputs_with_schema`].
pub fn compile_inputs_to_messagepack_with_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: &ProjectSchema,
) -> Result<MessagepackCompileReport, CompileError> {
    compile_inputs_to_messagepack_with_optional_schema(inputs, options, Some(schema))
}

fn compile_inputs_with_optional_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
) -> Result<CompileReport, CompileError> {
    let report = compile_inputs_with_output(
        inputs.into_iter().map(Input::Raw),
        options,
        schema,
        |dialogue| {
            let messagepack = serialize_messagepack(&dialogue)?;
            let inspection_json = serialize_inspection_json(&dialogue)?;
            Ok(CompiledAssetOutput {
                dialogue,
                messagepack,
                inspection_json,
            })
        },
    )?;
    Ok(CompileReport {
        diagnostics: report.diagnostics,
        asset: report.output,
    })
}

fn compile_inputs_to_messagepack_with_optional_schema(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
) -> Result<MessagepackCompileReport, CompileError> {
    let report = compile_inputs_with_output(
        inputs.into_iter().map(Input::Raw),
        options,
        schema,
        |dialogue| serialize_messagepack(&dialogue),
    )?;
    Ok(MessagepackCompileReport {
        diagnostics: report.diagnostics,
        messagepack: report.output,
    })
}
