//! `id check`: stable-ID diagnostics, and with `--since`, IDs removed since a
//! git revision such as the last release tag.
//!
//! Removing or renaming a shipped ID breaks translations and saves keyed on it.
//! The baseline is read from git rather than a sidecar so source stays the only
//! authority; a missing revision (for example, a shallow clone without tags)
//! is an error rather than an empty baseline.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use recite_core::DiagnosticCategory;
use recite_core::ast::{SourceFile, Statement};
use recite_parser::parse;

use crate::args::CheckIdsArgs;
use crate::diagnostics::{InputDiagnostics, report_diagnostics, report_targeted_diagnostics};
use crate::error::CliError;
use crate::fs::{collect_input_files, validate_inputs};
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
    let root = git(&anchor, &["rev-parse", "--show-toplevel"], None)
        .map_err(baseline_error)?
        .trim_end()
        .to_owned();
    let root = PathBuf::from(root);
    git(
        &root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
        None,
    )
    .map_err(|_| {
        baseline_error(
            "the revision is not available; fetch it (and its tags) before checking".to_owned(),
        )
    })?;

    let mut pathspecs = Vec::new();
    for path in paths {
        let absolute =
            std::path::absolute(path).map_err(|error| baseline_error(error.to_string()))?;
        let relative = absolute.strip_prefix(&root).map_err(|_| {
            baseline_error(format!("{} is outside the git repository", path.display()))
        })?;
        pathspecs.push(relative.to_string_lossy().into_owned());
    }
    let mut listing_args = vec!["ls-tree", "-r", "-z", "--name-only", reference, "--"];
    listing_args.extend(pathspecs.iter().map(String::as_str));
    let listing = git(&root, &listing_args, None).map_err(baseline_error)?;
    let files = listing
        .split('\0')
        .filter(|file| file.ends_with(".recite"))
        .collect::<Vec<_>>();

    let requests = files
        .iter()
        .map(|file| format!("{reference}:{file}\n"))
        .collect::<String>();
    let blobs = git(&root, &["cat-file", "--batch"], Some(&requests)).map_err(baseline_error)?;
    let mut ids = BTreeSet::new();
    for source in batch_contents(&blobs).map_err(baseline_error)? {
        collect_ids(&parse("", source).lower_source_file().source_file, &mut ids);
    }
    Ok(ids)
}

fn parent(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_owned)
}

/// Splits `git cat-file --batch` output: `<oid> <type> <size>\n<content>\n`.
fn batch_contents(output: &str) -> Result<Vec<&str>, String> {
    let mut contents = Vec::new();
    let mut rest = output;
    while !rest.is_empty() {
        let (header, after) = rest
            .split_once('\n')
            .ok_or_else(|| "truncated git object listing".to_owned())?;
        let size = header
            .rsplit(' ')
            .next()
            .and_then(|size| size.parse::<usize>().ok())
            .ok_or_else(|| format!("unexpected git object header `{header}`"))?;
        let content = after
            .get(..size)
            .ok_or_else(|| "truncated git object".to_owned())?;
        contents.push(content);
        rest = after.get(size + 1..).unwrap_or_default();
    }
    Ok(contents)
}

fn git(directory: &Path, args: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not run git: {error}"))?;
    // Feed stdin from its own thread so a large batch cannot fill both pipes.
    let writer = input.zip(child.stdin.take()).map(|(input, mut stdin)| {
        let input = input.to_owned();
        std::thread::spawn(move || stdin.write_all(input.as_bytes()))
    });
    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout)
            .map_err(|error| format!("could not read git output: {error}"))?;
    }
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| "git input writer panicked".to_owned())?
            .map_err(|error| format!("could not write to git: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not run git: {error}"))?;
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
