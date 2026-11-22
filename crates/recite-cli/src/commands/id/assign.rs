//! `id assign`: append generated stable IDs to headers that have none.
//!
//! Planning and editing belong to the shared authoring kernel, so the CLI
//! inserts exactly what the LSP quick fix would.

use std::collections::BTreeMap;
use std::io::Write;

use recite_compiler::authoring::{
    AuthoringEditError, AuthoringKernel, AuthoringRequest, SavedDocument,
};
use recite_core::{DocumentKey, StableIdGenerator};

use crate::args::AssignIdsArgs;
use crate::diagnostics::{Subject, report_diagnostics, report_success};
use crate::error::CliError;
use crate::fs::{LooseKeys, SourceSet, validate_inputs, write_staged};
use crate::i18n::{Messages, MsgId};
use recite_ui::UiArg;

pub(super) fn run(
    args: AssignIdsArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let mut paths = BTreeMap::new();
    let mut documents = Vec::new();
    let preview = args.dry_run || args.check;
    let check = args.check;
    let args = args.inputs;
    for input in SourceSet::read(&args.paths, LooseKeys::CommonDirectory)?.named {
        let key = DocumentKey::new(input.key).map_err(|error| CliError::Read {
            path: input.path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
        })?;
        documents.push(SavedDocument::new(key.clone(), input.source.clone()));
        paths.insert(key, (input.path, input.source));
    }

    let mut kernel = AuthoringKernel::new();
    let generation = kernel.snapshot().generation();
    kernel
        .apply(AuthoringRequest::new(generation, documents, []))
        .map_err(|_| CliError::Diagnostics)?;
    let snapshot = kernel.snapshot();
    let plan = match snapshot.plan_insert_missing_ids(&mut StableIdGenerator::from_system()) {
        Ok(plan) => plan,
        Err(AuthoringEditError::NoEdits) => {
            writeln!(stderr, "{}", messages.text(MsgId::CliIdsNoneMissing))?;
            return Ok(());
        }
        // Syntax errors and malformed IDs need an author decision first.
        Err(_) => {
            let diagnostics = validate_inputs(&args.paths, None)?.into_all();
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Err(CliError::Diagnostics);
        }
    };

    let assigned = plan.edits().len();
    if preview {
        write_planned(stderr, &plan, &paths)?;
        let summary = if check {
            MsgId::CliIdsMissingCheck
        } else {
            MsgId::CliIdsPlanned
        };
        writeln!(
            stderr,
            "{}",
            messages.format(summary, [("count", assigned)])
        )?;
        return if check {
            Err(CliError::Diagnostics)
        } else {
            Ok(())
        };
    }
    let mut written = Vec::new();
    for (key, text) in plan.apply(snapshot).map_err(|_| CliError::Diagnostics)? {
        if let Some((path, _)) = paths.get(&key) {
            write_staged(path, text.as_bytes())?;
            written.push(crate::diagnostics::display_path(&path.to_string_lossy()));
        }
    }
    let message = messages.format(
        MsgId::CliIdsAssigned,
        [
            ("count", UiArg::from(assigned)),
            ("files", UiArg::from(written.join(", "))),
        ],
    );
    report_success(stderr, messages, Subject::Message(message))?;
    Ok(())
}

/// One row per planned ID: where it goes, the ID, and the header it completes.
fn write_planned(
    stderr: &mut dyn Write,
    plan: &recite_compiler::authoring::AuthoringEditPlan,
    paths: &BTreeMap<DocumentKey, (std::path::PathBuf, String)>,
) -> Result<(), CliError> {
    for edit in plan.edits() {
        let Some((path, source)) = paths.get(edit.document()) else {
            continue;
        };
        let line = edit.range().start().line();
        let header = recite_core::source_lines(source)
            .nth(
                usize::try_from(line)
                    .unwrap_or(usize::MAX)
                    .saturating_sub(1),
            )
            .map_or("", |(content, _)| content.trim());
        writeln!(
            stderr,
            "  {}:{line}  {}  {header}",
            crate::fs::display_path(path),
            edit.replacement().trim()
        )?;
    }
    Ok(())
}
