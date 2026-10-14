//! Reuse requires exact captured inputs, never timestamps or partial discovery.
use recite_compiler::authoring::{
    BuildGeneration, BuildInput, BuildInputAuthority, BuildInputKind, BuildRequest,
    SnapshotGeneration,
};
use recite_config::ProjectDiscoveryReport;
use recite_core::{DocumentKey, schema::ProjectSchema};

use super::super::PROJECT_MANIFEST_FILE;
use crate::{ProjectBuildPreparationError, ProjectBuildRequest};

pub(super) fn unchanged(
    previous: &ProjectBuildRequest,
    discovery: &ProjectDiscoveryReport,
    schema: Option<&ProjectSchema>,
    schema_key: Option<&DocumentKey>,
) -> bool {
    // Ready diagnostics combine discovery, manifest and source validation.
    // Revalidate warning-bearing requests rather than retain stale warnings.
    if !previous.diagnostics().is_empty()
        || previous.project_root() != discovery.manifest().project_root()
        || previous.manifest() != discovery.manifest().source()
        || previous.schema() != schema
    {
        return false;
    }
    let inputs = previous.build_request().inputs();
    let captured_schema = inputs
        .iter()
        .find(|input| input.kind() == &BuildInputKind::Schema);
    if captured_schema.map(|input| input.key()) != schema_key {
        return false;
    }
    inputs
        .iter()
        .filter(|input| input.kind() == &BuildInputKind::Source)
        .map(|input| (input.key(), input.content()))
        .eq(discovery
            .documents()
            .iter()
            .map(|document| (document.key(), Some(document.text()))))
}

pub(super) fn capture(
    discovery: &ProjectDiscoveryReport,
    schema: Option<&ProjectSchema>,
    schema_key: Option<DocumentKey>,
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
) -> Result<BuildRequest, ProjectBuildPreparationError> {
    let manifest_key = DocumentKey::new(PROJECT_MANIFEST_FILE.to_owned()).map_err(|error| {
        ProjectBuildPreparationError::InvalidInputKey {
            key: PROJECT_MANIFEST_FILE.to_owned(),
            reason: error.to_string(),
        }
    })?;
    let mut inputs = Vec::with_capacity(discovery.documents().len() + 2);
    inputs.push(BuildInput::new(
        manifest_key,
        BuildInputKind::Manifest,
        BuildInputAuthority::Saved,
        discovery.manifest().source().source_text(),
    ));
    for document in discovery.documents() {
        inputs.push(BuildInput::new(
            document.key().clone(),
            BuildInputKind::Source,
            BuildInputAuthority::Saved,
            document.text(),
        ));
    }
    if let (Some(schema), Some(key)) = (schema, schema_key) {
        inputs.push(BuildInput::schema(
            key,
            BuildInputAuthority::Saved,
            schema.clone(),
        ));
    }

    Ok(BuildRequest::new_with_policy(
        generation,
        snapshot_generation,
        inputs,
        recite_compiler::authoring::BuildInputPolicy::SavedOnly,
    )?)
}
