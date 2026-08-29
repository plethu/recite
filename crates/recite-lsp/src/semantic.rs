use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use recite_core::SourceFile;
use recite_parser::parse;

use super::{DocumentDiagnostics, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn with_semantic_diagnostics(
        &self,
        mut diagnostics: DocumentDiagnostics,
    ) -> DocumentDiagnostics {
        if !diagnostics.diagnostics.is_empty() {
            return diagnostics;
        }
        let Some(source_files) = self.live_source_files() else {
            return diagnostics;
        };
        let validation_path = self.validation_path_for_uri(&diagnostics.uri);
        diagnostics.diagnostics = match self.schema.schema() {
            Some(schema) => {
                recite_compiler::validate_source_files_with_schema(&source_files, schema)
                    .diagnostics
            }
            None => recite_compiler::validate_source_files(&source_files).diagnostics,
        };
        diagnostics
            .diagnostics
            .retain(|diagnostic| diagnostic.span.file == validation_path);
        diagnostics
    }

    fn live_source_files(&self) -> Option<Vec<SourceFile>> {
        let open_saved_paths = self
            .documents
            .documents()
            .filter_map(|document| document.summary().saved_path().map(Path::to_owned))
            .collect::<BTreeSet<PathBuf>>();
        let open_uris = self
            .documents
            .documents()
            .map(|document| document.summary().uri().as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let mut inputs = self
            .saved
            .documents()
            .filter(|document| {
                !document
                    .summary
                    .saved_path()
                    .is_some_and(|path| open_saved_paths.contains(path))
                    && !open_uris.contains(document.summary.uri().as_str())
            })
            .map(|document| {
                (
                    document
                        .summary
                        .project_relative_path()
                        .unwrap_or(document.summary.uri().as_str())
                        .to_owned(),
                    document.text.as_str(),
                )
            })
            .collect::<Vec<_>>();
        inputs.extend(self.documents.documents().map(|document| {
            (
                document
                    .summary()
                    .project_relative_path()
                    .unwrap_or(document.identity().uri.as_str())
                    .to_owned(),
                document.text(),
            )
        }));
        inputs.sort_by(|left, right| left.0.cmp(&right.0));
        let mut source_files = Vec::with_capacity(inputs.len());
        for (uri, text) in inputs {
            let lowered = parse(uri.as_str(), text).lower_source_file();
            if !lowered.diagnostics.is_empty() {
                return None;
            }
            source_files.push(lowered.source_file);
        }
        Some(source_files)
    }
}
