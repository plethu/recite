//! Compiled assets to compare: a compiled file, a project built in memory, or
//! a project built as it was at a git revision.

use std::io::Write;
use std::path::Path;

use recite_build::{ProjectBuildEngine, ProjectBuildPreparation, ProjectBuildPreparationError};
use recite_compiler::authoring::{
    BuildControl, BuildEngine, BuildFailure, BuildGeneration, SnapshotGeneration,
};
use recite_config::discover_project;
use recite_core::compiled::{CompiledDialogue, decode_compiled_dialogue_messagepack};

use crate::diagnostics::report_diagnostics;
use crate::error::{CliError, SaveError};
use crate::git;
use crate::i18n::Messages;

/// Compiled assets keyed by asset ID, in build-target order.
pub(crate) type Assets = Vec<(String, CompiledDialogue)>;

/// A compiled `.recitec` file, or every asset a project directory builds.
pub(super) fn load(
    path: &Path,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<Assets, CliError> {
    if path.is_dir() {
        build_project(path, stderr, messages)
    } else {
        compiled_file(path)
            .map(|dialogue| vec![(dialogue.header.asset_id.as_str().to_owned(), dialogue)])
    }
}

pub(super) fn compiled_file(path: &Path) -> Result<CompiledDialogue, CliError> {
    let bytes = std::fs::read(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    decode_compiled_dialogue_messagepack(&bytes).map_err(|source| CliError::DecodeAsset {
        path: path.to_owned(),
        source,
    })
}

/// Builds the project at `project` as it was at `reference`, in a temporary
/// directory that is removed afterwards.
pub(super) fn project_at_revision(
    project: &Path,
    reference: &str,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<Assets, CliError> {
    let baseline_error = |detail: String| SaveError::Baseline {
        reference: reference.to_owned(),
        detail,
    };
    if !project.is_dir() {
        return Err(SaveError::SinceNeedsProject {
            candidate: crate::fs::display_path(project),
        }
        .into());
    }
    // Project content stays inside the manifest directory, which may be above
    // the directory the command was given.
    let discovery =
        discover_project(project).map_err(|source| CliError::ProjectDiscovery { source })?;
    let manifest_directory = discovery.manifest().project_root().to_owned();
    let root = git::repository_at(&manifest_directory, reference).map_err(baseline_error)?;
    let project_spec = git::pathspec(&root, &manifest_directory).map_err(baseline_error)?;
    // The manifest may declare schema, settings and catalogue files under any
    // name, so the whole project directory is checked out.
    let files = git::files_at(
        &root,
        reference,
        std::slice::from_ref(&project_spec),
        |_| true,
    )
    .map_err(baseline_error)?;
    if files.is_empty() {
        return Err(baseline_error("the project did not exist at that revision".to_owned()).into());
    }
    let checkout = tempfile::tempdir().map_err(CliError::Io)?;
    for (file, contents) in files {
        let destination = checkout.path().join(&file);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(CliError::Io)?;
        }
        std::fs::write(&destination, contents).map_err(CliError::Io)?;
    }
    let project_root = if project_spec == "." {
        checkout.path().to_owned()
    } else {
        checkout.path().join(&project_spec)
    };
    build_project(&project_root, stderr, messages)
}

/// Builds every scene asset of a project in memory without writing output.
fn build_project(
    root: &Path,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<Assets, CliError> {
    let discovery =
        discover_project(root).map_err(|source| CliError::ProjectDiscovery { source })?;
    let preparation = recite_build::prepare_discovered(
        discovery,
        BuildGeneration::initial(),
        SnapshotGeneration::initial(),
    )
    .map_err(preparation_error)?;
    let request = match preparation {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Err(CliError::Diagnostics);
        }
        _ => {
            return Err(SaveError::Build {
                detail: "project preparation returned an unknown outcome".to_owned(),
            }
            .into());
        }
    };
    let mut engine = ProjectBuildEngine::new(&request);
    let candidates = match engine.build(request.build_request(), &BuildControl::new()) {
        Ok(candidates) => candidates,
        Err(BuildFailure::Diagnostics { diagnostics }) => {
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Err(CliError::Diagnostics);
        }
        Err(failure) => {
            return Err(SaveError::Build {
                detail: failure.to_string(),
            }
            .into());
        }
    };
    candidates
        .iter()
        .map(|candidate| {
            let dialogue =
                decode_compiled_dialogue_messagepack(candidate.bytes()).map_err(|source| {
                    SaveError::Build {
                        detail: source.to_string(),
                    }
                })?;
            Ok((dialogue.header.asset_id.as_str().to_owned(), dialogue))
        })
        .collect()
}

fn preparation_error(error: ProjectBuildPreparationError) -> CliError {
    match error {
        ProjectBuildPreparationError::Discovery(source) => CliError::ProjectDiscovery { source },
        ProjectBuildPreparationError::NoInputs => CliError::NoInputs,
        other => SaveError::Build {
            detail: other.to_string(),
        }
        .into(),
    }
}
