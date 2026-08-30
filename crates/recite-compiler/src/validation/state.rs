use std::collections::{BTreeMap, BTreeSet};

use recite_core::{Block, Diagnostic, ProjectSchema, SourceFile, SourceSpan};

use super::ids::collect_line_ids;
use super::participation::{ValidationCompleteness, ValidationParticipation, ValidationSourceFile};
use super::project::{
    collect_blocks, first_source_span, first_validation_source_span,
    validation_source_files_in_project_order,
};
use crate::diagnostics;

pub(crate) struct Validator<'a> {
    pub(crate) source_files: Vec<ValidationSourceFile<'a>>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(super) schema: Option<&'a ProjectSchema>,
    pub(super) participation: super::participation::ValidationParticipation,
    pub(super) blocks: BTreeMap<&'a str, BTreeSet<&'a str>>,
    pub(super) block_definition_completeness: BTreeMap<&'a str, ValidationCompleteness>,
    pub(super) source_paths: BTreeMap<&'a str, SourceSpan>,
    pub(super) block_ids: BTreeMap<(&'a str, &'a str), SourceSpan>,
    pub(super) compiled_block_ids: BTreeMap<&'a str, (&'a str, SourceSpan)>,
    pub(super) line_ids: BTreeSet<&'a str>,
    pub(super) localisable_ids: BTreeMap<&'a str, SourceSpan>,
    pub(super) first_default: Option<&'a Block>,
    pub(super) default_count: usize,
}

impl<'a> Validator<'a> {
    pub(crate) fn new(
        source_files: &[ValidationSourceFile<'a>],
        schema: Option<&'a ProjectSchema>,
    ) -> Self {
        let source_files = validation_source_files_in_project_order(source_files);
        let blocks = collect_blocks(&source_files);
        let line_ids = collect_line_ids(&source_files);
        let mut block_definition_completeness = BTreeMap::new();
        for source_file in &source_files {
            let path = source_file.source_file.path.as_str();
            let entry = block_definition_completeness
                .entry(path)
                .or_insert(source_file.participation.block_definitions);
            if source_file.participation.block_definitions == ValidationCompleteness::Incomplete {
                *entry = ValidationCompleteness::Incomplete;
            }
        }

        Self {
            source_files,
            diagnostics: Vec::new(),
            schema,
            participation: ValidationParticipation::all_complete(),
            blocks,
            block_definition_completeness,
            source_paths: BTreeMap::new(),
            block_ids: BTreeMap::new(),
            compiled_block_ids: BTreeMap::new(),
            line_ids,
            localisable_ids: BTreeMap::new(),
            first_default: None,
            default_count: 0,
        }
    }
    #[cfg(feature = "bench-support")]
    pub(crate) fn for_block_reference_probe(source_files: &'a [SourceFile]) -> Self {
        let source_files = source_files
            .iter()
            .map(ValidationSourceFile::all_complete)
            .collect::<Vec<_>>();
        let source_files = validation_source_files_in_project_order(&source_files);
        let blocks = collect_blocks(&source_files);
        let block_definition_completeness = source_files
            .iter()
            .map(|source_file| {
                (
                    source_file.source_file.path.as_str(),
                    ValidationCompleteness::Complete,
                )
            })
            .collect();
        Self {
            source_files,
            blocks,
            block_definition_completeness,
            ..Self::empty_probe_state(None)
        }
    }
    #[cfg(feature = "bench-support")]
    pub(crate) fn for_localisable_id_probe(source_files: &'a [SourceFile]) -> Self {
        let source_files = source_files
            .iter()
            .map(ValidationSourceFile::all_complete)
            .collect::<Vec<_>>();
        let source_files = validation_source_files_in_project_order(&source_files);
        Self {
            source_files,
            ..Self::empty_probe_state(None)
        }
    }
    #[cfg(feature = "bench-support")]
    pub(crate) fn for_markup_probe(
        source_files: &'a [SourceFile],
        schema: &'a ProjectSchema,
    ) -> Self {
        let source_files = source_files
            .iter()
            .map(ValidationSourceFile::all_complete)
            .collect::<Vec<_>>();
        let source_files = validation_source_files_in_project_order(&source_files);
        Self {
            source_files,
            ..Self::empty_probe_state(Some(schema))
        }
    }
    #[cfg(feature = "bench-support")]
    fn empty_probe_state(schema: Option<&'a ProjectSchema>) -> Self {
        Self {
            source_files: Vec::new(),
            diagnostics: Vec::new(),
            schema,
            participation: ValidationParticipation::all_complete(),
            blocks: BTreeMap::new(),
            block_definition_completeness: BTreeMap::new(),
            source_paths: BTreeMap::new(),
            block_ids: BTreeMap::new(),
            compiled_block_ids: BTreeMap::new(),
            line_ids: BTreeSet::new(),
            localisable_ids: BTreeMap::new(),
            first_default: None,
            default_count: 0,
        }
    }
    pub(super) fn validate(&mut self) {
        for source_file in self.source_files.clone() {
            self.validate_source_file(source_file);
        }

        let all_block_definitions_complete = self.source_files.iter().all(|source_file| {
            source_file.participation.block_definitions == ValidationCompleteness::Complete
        });
        if self.default_count == 0
            && !self.source_files.is_empty()
            && all_block_definitions_complete
        {
            self.diagnostics.push(diagnostics::missing_default_block(
                first_validation_source_span(&self.source_files),
            ));
        }
    }
    pub(super) fn validate_source_file(&mut self, input: ValidationSourceFile<'a>) {
        let source_file = input.source_file;
        self.participation = input.participation;
        self.validate_source_path(source_file);

        for block in &source_file.blocks {
            self.validate_block(source_file, block);
            for statement in &block.statements {
                self.validate_statement_with_block(
                    source_file,
                    statement,
                    block
                        .default_speaker
                        .as_ref()
                        .map(|speaker| speaker.as_str()),
                );
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
