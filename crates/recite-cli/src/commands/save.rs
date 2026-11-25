//! `save`: players' saved sessions across dialogue patches.

use std::io::Write;
use std::path::Path;

use crate::args::SaveCommand;
use crate::error::CliError;
use crate::i18n::Messages;

mod assets;
mod impact;
mod inspect;

pub(crate) use assets::Assets;

/// Baseline assets for `watch --save-baseline`: a compiled asset file when
/// `spec` names one, otherwise a git revision of the project.
pub(crate) fn load_baseline(
    project_root: &Path,
    spec: &str,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<Assets, CliError> {
    let path = Path::new(spec);
    if path.is_file() {
        assets::load(path, stderr, messages)
    } else {
        assets::project_at_revision(project_root, spec, stderr, messages)
    }
}

/// The published scene assets of a project, as `watch` last wrote them.
pub(crate) fn published_assets(
    project_root: &Path,
    manifest: &recite_config::ProjectManifest,
) -> Result<Assets, CliError> {
    manifest
        .source()
        .manifest()
        .scenes
        .iter()
        .map(|scene| scene.asset.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|asset| {
            let dialogue = assets::compiled_file(&project_root.join(asset))?;
            Ok((dialogue.header.asset_id.as_str().to_owned(), dialogue))
        })
        .collect()
}

/// Writes the readable save-impact report of `candidates` against `baselines`.
pub(crate) fn write_impact(
    out: &mut dyn Write,
    messages: &Messages,
    baselines: &Assets,
    candidates: &Assets,
) -> Result<(), CliError> {
    impact::write_reports(out, messages, baselines, candidates)
}

pub(super) fn run(
    command: SaveCommand,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        SaveCommand::Impact(args) => impact::run(&args, stdout, stderr, messages),
        SaveCommand::Inspect(args) => inspect::run(&args, stdout, messages),
    }
}
