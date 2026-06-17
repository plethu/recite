use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use recite_core::{ProjectManifest, decode_compiled_dialogue_messagepack};
use recite_parser::parse;

use super::{
    BenchCounts, BenchGroup, BenchOperationReport, BenchTargetKind, BenchTargetReport,
    TargetMetadata, timed_operation,
};
use crate::{BenchmarkResult, error};

const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";

pub(super) fn build_project_root_reports(
    project_root: &Path,
    groups: &[BenchGroup],
    samples: usize,
) -> BenchmarkResult<Vec<BenchTargetReport>> {
    let unsupported = groups
        .iter()
        .filter(|group| !matches!(group, BenchGroup::Compiler))
        .map(|group| group.as_str())
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        return Err(error(format!(
            "project-root benchmark mode currently supports compiler group only; unsupported groups: {}",
            unsupported.join(", ")
        )));
    }

    let project = ProjectRootBenchProject::load(project_root)?;
    let operations = project_root_operations(&project, samples)?;
    Ok(vec![BenchTargetReport {
        target: project.display_root.clone(),
        kind: BenchTargetKind::ProjectRoot,
        metadata: TargetMetadata {
            fixture: None,
            project_root: Some(project.display_root.clone()),
            counts: project.counts.clone(),
            notes: vec![
                "Project-root mode measures manifest, compiled asset decode, and source parse/load surfaces.".to_owned(),
                "Runtime traversal remains fixture-only until a project runtime fixture contract exists.".to_owned(),
            ],
        },
        operations,
    }])
}

fn project_root_operations(
    project: &ProjectRootBenchProject,
    samples: usize,
) -> BenchmarkResult<Vec<BenchOperationReport>> {
    Ok(vec![
        timed_operation(
            BenchGroup::Compiler,
            "project_manifest_load",
            samples,
            || {
                let report = ProjectManifest::load_str(
                    project.manifest_path.to_string_lossy(),
                    &project.manifest_source,
                );
                if !report.diagnostics.is_empty() {
                    return Err(error(format!(
                        "project manifest produced {} diagnostics",
                        report.diagnostics.len()
                    )));
                }
                std::hint::black_box(report.manifest);
                Ok(())
            },
        )?,
        timed_operation(
            BenchGroup::Compiler,
            "project_asset_decode",
            samples,
            || {
                for asset in &project.assets {
                    let dialogue =
                        decode_compiled_dialogue_messagepack(std::hint::black_box(&asset.bytes))
                            .map_err(|decode_error| {
                                error(format!(
                                    "failed to decode compiled project asset: {decode_error}"
                                ))
                            })?;
                    std::hint::black_box(dialogue);
                }
                Ok(())
            },
        )?,
        timed_operation(
            BenchGroup::Compiler,
            "project_source_parse",
            samples,
            || {
                for source in &project.sources {
                    let parsed = parse(
                        std::hint::black_box(&source.path),
                        std::hint::black_box(&source.source),
                    );
                    if !parsed.diagnostics().is_empty() {
                        return Err(error(format!(
                            "project source `{}` produced {} parse diagnostics",
                            source.path,
                            parsed.diagnostics().len()
                        )));
                    }
                    std::hint::black_box(parsed);
                }
                Ok(())
            },
        )?,
    ])
}

#[derive(Clone, Debug)]
struct ProjectRootBenchProject {
    display_root: String,
    manifest_path: PathBuf,
    manifest_source: String,
    assets: Vec<ProjectAsset>,
    sources: Vec<ProjectSource>,
    counts: BenchCounts,
}

#[derive(Clone, Debug)]
struct ProjectAsset {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct ProjectSource {
    path: String,
    source: String,
}

impl ProjectRootBenchProject {
    fn load(project_root: &Path) -> BenchmarkResult<Self> {
        let manifest_path = project_root.join(PROJECT_MANIFEST_FILE);
        let manifest_source = fs::read_to_string(&manifest_path)?;
        let manifest_report =
            ProjectManifest::load_str(manifest_path.to_string_lossy(), &manifest_source);
        if !manifest_report.diagnostics.is_empty() {
            return Err(error(format!(
                "project manifest produced {} diagnostics",
                manifest_report.diagnostics.len()
            )));
        }
        let manifest = manifest_report
            .manifest
            .ok_or_else(|| error("project manifest did not load"))?;
        let mut assets = Vec::new();
        let mut source_paths = BTreeSet::new();
        let mut counts = BenchCounts {
            schema_files: u64::from(manifest.project.schema.is_some()),
            ..BenchCounts::default()
        };
        for scene in &manifest.scenes {
            let asset_path = project_root.join(&scene.asset);
            let bytes = fs::read(&asset_path)?;
            let dialogue =
                decode_compiled_dialogue_messagepack(&bytes).map_err(|decode_error| {
                    error(format!(
                        "failed to decode compiled project asset `{}`: {decode_error}",
                        asset_path.display()
                    ))
                })?;
            counts.compiled_asset_bytes =
                Some(counts.compiled_asset_bytes.unwrap_or(0) + bytes.len() as u64);
            counts.source_files += dialogue.sources.len() as u64;
            counts.blocks += dialogue.blocks.len() as u64;
            counts.dialogue_lines += dialogue.lines.len() as u64;
            counts.choices += dialogue.choices.len() as u64;
            counts.effects += dialogue.effects.len() as u64;
            counts.conditions += dialogue.condition_availability_reasons.len() as u64;
            for source in &dialogue.sources {
                let resolved_source =
                    project_source_candidates(project_root, &asset_path, &source.path)
                        .into_iter()
                        .find(|path| path.is_file())
                        .unwrap_or_else(|| project_root.join(&source.path));
                source_paths.insert(resolved_source);
            }
            assets.push(ProjectAsset { bytes });
        }

        let mut sources = Vec::new();
        let mut source_bytes = 0_u64;
        let mut recite_lines = 0_u64;
        for path in source_paths {
            let source = fs::read_to_string(&path)?;
            source_bytes += source.len() as u64;
            recite_lines += source.lines().count() as u64;
            sources.push(ProjectSource {
                path: path.to_string_lossy().replace('\\', "/"),
                source,
            });
        }
        counts.source_files = sources.len() as u64;
        counts.recite_lines = recite_lines;
        counts.project_bytes = Some(source_bytes + manifest_source.len() as u64);

        Ok(Self {
            display_root: project_root.to_string_lossy().replace('\\', "/"),
            manifest_path,
            manifest_source,
            assets,
            sources,
            counts,
        })
    }
}

fn project_source_candidates(
    project_root: &Path,
    asset_path: &Path,
    source_path: &str,
) -> Vec<PathBuf> {
    let source_path = Path::new(source_path);
    if source_path.is_absolute() {
        return vec![source_path.to_owned()];
    }

    let mut candidates = Vec::new();
    let mut ancestor = asset_path.parent();
    while let Some(directory) = ancestor {
        let candidate = directory.join(source_path);
        if !candidates.iter().any(|existing| existing == &candidate) {
            candidates.push(candidate);
        }

        if directory == project_root {
            break;
        }
        ancestor = directory.parent();
    }

    let project_candidate = project_root.join(source_path);
    if !candidates
        .iter()
        .any(|existing| existing == &project_candidate)
    {
        candidates.push(project_candidate);
    }

    candidates
}
