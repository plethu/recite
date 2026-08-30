use recite_compiler::{
    BuildCandidate, BuildCheck, BuildControl, BuildEngine, BuildFailure, BuildFailureReason,
    BuildInputKind, BuildRequest, CompileInput, CompileOptions,
};
use recite_core::{CompiledAssetId, CompilerVersion, Diagnostic, SchemaFingerprint, SourceMapId};

use super::request::ProjectBuildRequest;

/// A compiler-backed engine that keeps every candidate in memory.
///
/// This adapter owns no filesystem publisher and never writes project output.
/// Preparation has already established the manifest, schema, source, and
/// discovery validation boundary; the engine only compiles those immutable
/// inputs into deterministic candidates for a later host publisher.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ProjectBuildEngine {
    targets: Vec<super::request::ProjectBuildTarget>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectBuildEngine {
    /// Create an engine for a prepared project request.
    #[must_use]
    pub fn new(request: &ProjectBuildRequest) -> Self {
        Self {
            targets: request.targets().to_vec(),
            diagnostics: request.diagnostics().to_vec(),
        }
    }
}

impl BuildEngine for ProjectBuildEngine {
    fn check(&mut self, request: &BuildRequest, _control: &BuildControl) -> BuildCheck {
        BuildCheck::new(
            request,
            self.diagnostics.clone(),
            recite_compiler::FreshnessAssessment::fresh(request.fingerprints().clone()),
        )
    }

    fn build(
        &mut self,
        request: &BuildRequest,
        control: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        let schema = request
            .inputs()
            .iter()
            .find(|input| input.kind() == &BuildInputKind::Schema)
            .and_then(|input| input.schema_model());
        let inputs = source_inputs(request);
        let mut candidates = Vec::with_capacity(self.targets.len());

        for target in &self.targets {
            if control.cancellation().is_some() {
                break;
            }
            let options = compile_options(target.asset_id(), request.fingerprints().schema())?;
            let report = match schema {
                Some(schema) => {
                    recite_compiler::compile_inputs_with_schema(inputs.clone(), options, schema)
                }
                None => recite_compiler::compile_inputs(inputs.clone(), options),
            }
            .map_err(|_| BuildFailure::Engine {
                reason: BuildFailureReason::InvalidOutput,
            })?;
            if !report.diagnostics.is_empty() {
                return Err(BuildFailure::Diagnostics {
                    diagnostics: report.diagnostics,
                });
            }
            let Some(asset) = report.asset else {
                return Err(BuildFailure::Engine {
                    reason: BuildFailureReason::InvalidOutput,
                });
            };
            candidates.push(BuildCandidate::new(
                target.target().clone(),
                asset.messagepack,
            ));
            if control.cancellation().is_some() {
                break;
            }
        }

        Ok(candidates)
    }
}

fn source_inputs(request: &BuildRequest) -> Vec<CompileInput> {
    request
        .inputs()
        .iter()
        .filter(|input| input.kind() == &BuildInputKind::Source)
        .filter_map(|input| {
            input
                .content()
                .map(|source| CompileInput::new(input.key().as_str(), source))
        })
        .collect()
}

fn compile_options(
    asset_id: &str,
    schema_fingerprint: &SchemaFingerprint,
) -> Result<CompileOptions, BuildFailure> {
    let compiler_version =
        CompilerVersion::new(env!("CARGO_PKG_VERSION")).map_err(|_| BuildFailure::Engine {
            reason: BuildFailureReason::InvalidOutput,
        })?;
    let asset_id = CompiledAssetId::new(asset_id.to_owned()).map_err(|_| BuildFailure::Engine {
        reason: BuildFailureReason::InvalidOutput,
    })?;
    let source_map_id = SourceMapId::new(format!("{}.map", asset_id.as_str())).map_err(|_| {
        BuildFailure::Engine {
            reason: BuildFailureReason::InvalidOutput,
        }
    })?;
    Ok(CompileOptions::new(
        compiler_version,
        asset_id,
        source_map_id,
        schema_fingerprint.clone(),
    ))
}
