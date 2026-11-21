//! `prose check`: spelling and grammar in dialogue and choice prose.
//!
//! Each input reads the `[prose]` settings of the nearest project manifest;
//! files outside a project use the defaults. Findings are warnings, so this
//! explicit check is the only command whose exit status they affect.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_config::{ProjectDiscoveryError, discover_project};
use recite_core::Diagnostic;
use recite_core::project::ProjectProse;
use recite_prose::{ProseChecker, ProseEdit, ProseSettings};

use crate::args::{ProseCheckArgs, ProseCommand};
use crate::diagnostics::{Subject, report_diagnostics, report_success};
use crate::error::CliError;
use crate::fs::{collect_input_files, display_path};
use crate::i18n::Messages;

pub(super) fn run(
    command: ProseCommand,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        ProseCommand::Check(args) => check(&args, stderr, messages),
    }
}

fn check(
    args: &ProseCheckArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let report = check_prose(&args.paths)?;
    if report.diagnostics.is_empty() {
        return report_success(stderr, messages, Subject::Files(report.file_count));
    }
    report_diagnostics(stderr, messages, report.diagnostics.iter())?;
    Err(CliError::Diagnostics)
}

/// Prose diagnostics for a set of inputs, with the fixes each one offers.
pub(crate) struct ProseReport {
    pub(crate) file_count: usize,
    /// Setup errors first, then findings in file and source order.
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// `(diagnostic index, file, edit)` for every offered fix.
    pub(crate) fixes: Vec<(usize, String, ProseEdit)>,
}

pub(crate) fn check_prose(paths: &[PathBuf]) -> Result<ProseReport, CliError> {
    let files = collect_input_files(paths)?;
    let mut projects = BTreeMap::<PathBuf, Option<Project>>::new();
    let mut checkers = BTreeMap::<Option<PathBuf>, Result<ProseChecker, Option<Diagnostic>>>::new();
    let mut setup = Vec::new();
    let mut findings = Vec::new();
    for file in &files {
        let directory = file.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        if !projects.contains_key(&directory) {
            projects.insert(directory.clone(), project_for(&directory)?);
        }
        let project = projects.get(&directory).and_then(Option::as_ref);
        let key = project.map(|project| project.manifest_path.clone());
        let checker = checkers.entry(key).or_insert_with(|| checker_for(project));
        let checker = match checker {
            Ok(checker) => checker,
            Err(diagnostic) => {
                if let Some(diagnostic) = diagnostic
                    && !setup.contains(diagnostic)
                {
                    setup.push(diagnostic.clone());
                }
                continue;
            }
        };
        let source = std::fs::read_to_string(file).map_err(|source| CliError::Read {
            path: file.clone(),
            source,
        })?;
        let path = display_path(file);
        findings.extend(
            checker
                .check(&path, &source)
                .into_iter()
                .map(|finding| (path.clone(), finding)),
        );
    }
    let mut diagnostics = setup;
    let mut fixes = Vec::new();
    for (path, finding) in findings {
        let index = diagnostics.len();
        fixes.extend(
            finding
                .fixes
                .iter()
                .map(|fix| (index, path.clone(), fix.clone())),
        );
        diagnostics.push(finding.diagnostic);
    }
    Ok(ProseReport {
        file_count: files.len(),
        diagnostics,
        fixes,
    })
}

struct Project {
    root: PathBuf,
    manifest_path: PathBuf,
    prose: ProjectProse,
}

fn project_for(directory: &Path) -> Result<Option<Project>, CliError> {
    let start = if directory.as_os_str().is_empty() {
        Path::new(".")
    } else {
        directory
    };
    match discover_project(start) {
        Ok(report) => {
            let manifest = report.manifest();
            Ok(Some(Project {
                root: manifest.project_root().to_owned(),
                manifest_path: manifest.manifest_path().to_owned(),
                prose: manifest.source().manifest().prose.clone(),
            }))
        }
        Err(ProjectDiscoveryError::NotFound { .. }) => Ok(None),
        Err(source) => Err(CliError::ProjectDiscovery { source }),
    }
}

fn checker_for(project: Option<&Project>) -> Result<ProseChecker, Option<Diagnostic>> {
    let (settings, manifest) = match project {
        Some(project) => (
            ProseSettings::from_project(&project.root, &project.prose),
            display_path(&project.manifest_path),
        ),
        None => (Ok(ProseSettings::default()), String::new()),
    };
    settings.and_then(ProseChecker::new).map_err(|error| {
        let file = if manifest.is_empty() {
            recite_config::PROJECT_MANIFEST_FILE
        } else {
            &manifest
        };
        error.diagnostic(file)
    })
}
