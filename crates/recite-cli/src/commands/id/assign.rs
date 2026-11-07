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

use crate::args::InputArgs;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::{read_relative_inputs, validate_inputs, write_staged};
use crate::i18n::{Messages, MsgId};

pub(super) fn run(
    args: InputArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let mut paths = BTreeMap::new();
    let mut documents = Vec::new();
    for input in read_relative_inputs(&args.paths)? {
        let key = DocumentKey::new(input.key).map_err(|error| CliError::Read {
            path: input.path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
        })?;
        documents.push(SavedDocument::new(key.clone(), input.source));
        paths.insert(key, input.path);
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
    for (key, text) in plan.apply(snapshot).map_err(|_| CliError::Diagnostics)? {
        if let Some(path) = paths.get(&key) {
            write_staged(path, text.as_bytes())?;
        }
    }
    writeln!(
        stderr,
        "{}",
        messages.format(MsgId::CliIdsAssigned, [("count", assigned)])
    )?;
    Ok(())
}
