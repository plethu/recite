//! `id check`: stable-ID diagnostics, and with `--since`, IDs removed since a
//! git revision such as the last release tag.
//!
//! Removing or renaming a shipped ID breaks translations and saves keyed on it.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_core::DiagnosticCategory;
use recite_core::ast::{SourceFile, Statement};
use recite_parser::parse;

use crate::args::CheckIdsArgs;
use crate::diagnostics::{InputDiagnostics, report_diagnostics, report_targeted_diagnostics};
use crate::error::CliError;
use crate::fs::{collect_input_files, validate_inputs};
use crate::git;
use crate::i18n::{Messages, MsgId};

pub(super) fn run(
    args: &CheckIdsArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let diagnostics = validate_inputs(&args.inputs.paths, None)?;
    let Some(reference) = &args.since else {
        return report_targeted_diagnostics(
            stderr,
            messages,
            diagnostics,
            DiagnosticCategory::Identifier,
        );
    };

    let current = current_ids(&args.inputs.paths)?;
    let removed = baseline_ids(&args.inputs.paths, reference)?
        .into_iter()
        .filter(|id| !current.contains(id) && !args.allow_removed.contains(id))
        .collect::<Vec<_>>();
    if removed.is_empty() {
        return report_targeted_diagnostics(
            stderr,
            messages,
            diagnostics,
            DiagnosticCategory::Identifier,
        );
    }
    for id in &removed {
        writeln!(
            stderr,
            "{}",
            messages.format(
                MsgId::CliIdsRemoved,
                [("id", id.clone()), ("reference", reference.clone())]
            )
        )?;
    }
    report_failures_only(stderr, messages, diagnostics)?;
    Err(CliError::Diagnostics)
}

/// Reports ID diagnostics without the success line, which would contradict
/// the removed-ID failure already printed.
fn report_failures_only(
    stderr: &mut dyn Write,
    messages: &Messages,
    diagnostics: InputDiagnostics,
) -> Result<(), CliError> {
    let targeted = diagnostics.into_category(DiagnosticCategory::Identifier);
    if !targeted.is_empty() {
        report_diagnostics(stderr, messages, targeted.iter())?;
    }
    Ok(())
}

fn current_ids(paths: &[PathBuf]) -> Result<BTreeSet<String>, CliError> {
    let mut ids = BTreeSet::new();
    for path in collect_input_files(paths)? {
        let source = std::fs::read_to_string(&path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        collect_ids(
            &parse("", &source).lower_source_file().source_file,
            &mut ids,
        );
    }
    Ok(ids)
}

fn collect_ids(file: &SourceFile, ids: &mut BTreeSet<String>) {
    file.visit_statements_depth_first(&mut |statement| {
        let source_id = match statement {
            Statement::Line(line) => &line.source_id,
            Statement::Choice(choice) => &choice.source_id,
            _ => return,
        };
        if let Some(id) = source_id.stable_id() {
            ids.insert(id.as_str().to_owned());
        }
    });
}

/// Stable IDs in the `.recite` files under `paths` as they were at `reference`.
fn baseline_ids(paths: &[PathBuf], reference: &str) -> Result<BTreeSet<String>, CliError> {
    let baseline_error = |detail: String| CliError::StableIdBaseline {
        reference: reference.to_owned(),
        detail,
    };
    let anchor = paths
        .first()
        .map(|path| {
            if path.is_dir() {
                path.clone()
            } else {
                parent(path)
            }
        })
        .unwrap_or_default();
    let root = git::repository_at(&anchor, reference).map_err(baseline_error)?;
    let pathspecs = paths
        .iter()
        .map(|path| git::pathspec(&root, path))
        .collect::<Result<Vec<_>, _>>()
        .map_err(baseline_error)?;
    let mut ids = BTreeSet::new();
    let sources = git::files_at(&root, reference, &pathspecs, |file| {
        file.ends_with(".recite")
    })
    .map_err(baseline_error)?;
    for (file, contents) in sources {
        let source = String::from_utf8(contents)
            .map_err(|_| baseline_error(format!("{file} is not valid UTF-8")))?;
        collect_ids(
            &parse("", &source).lower_source_file().source_file,
            &mut ids,
        );
    }
    Ok(ids)
}

fn parent(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_owned)
}
