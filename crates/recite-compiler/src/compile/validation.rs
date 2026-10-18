//! Diagnostic-only validation of immutable captured build inputs.
use super::pipeline::{Input, prepare_inputs};
use crate::authoring::{BuildInputKind, BuildRequest};
use recite_core::Diagnostic;

/// Validate captured source and schema without building analysis summaries or an asset.
/// Lowered syntax is released before returning; requests retain only captured inputs.
pub fn validate_build_request(request: &BuildRequest) -> Vec<Diagnostic> {
    let schema = request
        .inputs()
        .iter()
        .find_map(|input| input.schema_model());
    let inputs = request
        .inputs()
        .iter()
        .filter(|input| input.kind() == &BuildInputKind::Source)
        .map(Input::Captured);
    prepare_inputs(inputs, schema).diagnostics
}
