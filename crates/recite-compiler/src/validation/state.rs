use std::collections::{BTreeMap, BTreeSet};

use recite_core::{Block, Diagnostic, ProjectSchema, SourceFile, SourceSpan};

use super::ids::collect_line_ids;
use super::project::{collect_blocks, first_source_span, source_files_in_project_order};
use crate::diagnostics;

pub(super) struct Validator<'a> {
    pub(super) source_files: Vec<&'a SourceFile>,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) schema: Option<&'a ProjectSchema>,
    pub(super) blocks: BTreeMap<&'a str, BTreeSet<&'a str>>,
    pub(super) source_paths: BTreeMap<&'a str, SourceSpan>,
    pub(super) block_ids: BTreeMap<(&'a str, &'a str), SourceSpan>,
    pub(super) compiled_block_ids: BTreeMap<&'a str, (&'a str, SourceSpan)>,
    pub(super) line_ids: BTreeSet<&'a str>,
    pub(super) localisable_ids: BTreeMap<&'a str, SourceSpan>,
    pub(super) first_default: Option<&'a Block>,
    pub(super) default_count: usize,
}

impl<'a> Validator<'a> {
    pub(super) fn new(source_files: &'a [SourceFile], schema: Option<&'a ProjectSchema>) -> Self {
        let source_files = source_files_in_project_order(source_files);
        let blocks = collect_blocks(&source_files);
        let line_ids = collect_line_ids(&source_files);

        Self {
            source_files,
            diagnostics: Vec::new(),
            schema,
            blocks,
            source_paths: BTreeMap::new(),
            block_ids: BTreeMap::new(),
            compiled_block_ids: BTreeMap::new(),
            line_ids,
            localisable_ids: BTreeMap::new(),
            first_default: None,
            default_count: 0,
        }
    }
    pub(super) fn validate(&mut self) {
        for source_file in self.source_files.clone() {
            self.validate_source_file(source_file);
        }

        if self.default_count == 0 && !self.source_files.is_empty() {
            self.diagnostics
                .push(diagnostics::missing_default_block(first_source_span(
                    &self.source_files,
                )));
        }
    }
    pub(super) fn validate_source_file(&mut self, source_file: &'a SourceFile) {
        self.validate_source_path(source_file);

        for block in &source_file.blocks {
            self.validate_block(source_file, block);
            for statement in &block.statements {
                self.validate_statement(source_file, statement);
            }
        }
    }
    pub(super) fn validate_source_path(&mut self, source_file: &'a SourceFile) {
        let span = first_source_span(&[source_file]);
        if let Some(first_span) = self.source_paths.get(source_file.path.as_str()) {
            self.diagnostics.push(diagnostics::duplicate_source_path(
                source_file,
                first_span.clone(),
            ));
        } else {
            self.source_paths.insert(source_file.path.as_str(), span);
        }
    }
}
