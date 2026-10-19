//! Model and translation template from one fully checked compilation.
use super::{
    CompileError, CompileInput, CompileOptions,
    pipeline::{Input, compile_inputs_with_output},
};
use recite_core::{Diagnostic, compiled::CompiledDialogue, po::PotDocument, schema::ProjectSchema};

/// A runtime model and its source-owned localisation context.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CataloguedModel {
    pub dialogue: CompiledDialogue,
    pub catalogue: PotDocument,
}

/// Complete source/schema diagnostics prevent both outputs.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CataloguedModelCompileReport {
    pub diagnostics: Vec<Diagnostic>,
    pub output: Option<CataloguedModel>,
}

/// Compile a model and extract its POT without repeating lowering or validation.
///
/// Source ordering, translator context, diagnostics and model tables match the
/// separate model and POT entry points. Encoding remains the caller's responsibility.
pub fn compile_inputs_with_catalogue(
    inputs: impl IntoIterator<Item = CompileInput>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
) -> Result<CataloguedModelCompileReport, CompileError> {
    let report = compile_inputs_with_output(
        inputs.into_iter().map(Input::Raw),
        options,
        schema,
        |dialogue, inputs| {
            Ok(CataloguedModel {
                dialogue,
                catalogue: crate::pot::collect_pot(
                    inputs.iter().map(|input| &input.source_file),
                    schema,
                ),
            })
        },
    )?;
    Ok(CataloguedModelCompileReport {
        diagnostics: report.diagnostics,
        output: report.output,
    })
}
