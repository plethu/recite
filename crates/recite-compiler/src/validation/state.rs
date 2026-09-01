use std::collections::{BTreeMap, BTreeSet};

use recite_core::{Block, Diagnostic, ProjectSchema, SourceFile, SourceSpan};

use super::ids::collect_line_ids;
use super::participation::{
    ValidationCompleteness, ValidationInput, ValidationParticipation, aggregate_participation,
};
use super::project::{
    collect_blocks, first_source_span, first_validation_source_span,
    sort_validation_source_files_in_project_order,
};
use crate::diagnostics;

pub(crate) struct Validator<'a> {
    pub(crate) source_files: Vec<ValidationInput<'a>>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(super) schema: Option<&'a ProjectSchema>,
    pub(super) project_complete: bool,
    pub(super) participation: super::participation::ValidationParticipation,
    pub(super) blocks: BTreeMap<&'a str, BTreeSet<&'a str>>,
    pub(super) effective_participation: BTreeMap<&'a str, ValidationParticipation>,
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
        source_files: impl IntoIterator<Item = ValidationInput<'a>>,
        schema: Option<&'a ProjectSchema>,
        project_complete: bool,
    ) -> Self {
        let mut source_files = source_files.into_iter().collect::<Vec<_>>();
        sort_validation_source_files_in_project_order(&mut source_files);
        let effective_participation = aggregate_participation(&source_files);
        let blocks = collect_blocks(&source_files, &effective_participation);
        let line_ids = collect_line_ids(&source_files, &effective_participation);

        Self {
            source_files,
            diagnostics: Vec::new(),
            schema,
            project_complete,
            participation: ValidationParticipation::all_complete(),
            blocks,
            effective_participation,
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
        let mut source_files = source_files
            .iter()
            .map(ValidationInput::all_complete)
            .collect::<Vec<_>>();
        sort_validation_source_files_in_project_order(&mut source_files);
        let effective_participation = aggregate_participation(&source_files);
        let blocks = collect_blocks(&source_files, &effective_participation);
        Self {
            source_files,
            blocks,
            effective_participation,
            ..Self::empty_probe_state(None)
        }
    }
    #[cfg(feature = "bench-support")]
    pub(crate) fn for_localisable_id_probe(source_files: &'a [SourceFile]) -> Self {
        let mut source_files = source_files
            .iter()
            .map(ValidationInput::all_complete)
            .collect::<Vec<_>>();
        sort_validation_source_files_in_project_order(&mut source_files);
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
        let mut source_files = source_files
            .iter()
            .map(ValidationInput::all_complete)
            .collect::<Vec<_>>();
        sort_validation_source_files_in_project_order(&mut source_files);
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
            project_complete: true,
            participation: ValidationParticipation::all_complete(),
            blocks: BTreeMap::new(),
            effective_participation: BTreeMap::new(),
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
            self.effective_participation
                .get(source_file.source_file().path.as_str())
                .is_some_and(|participation| {
                    participation.block_definitions() == ValidationCompleteness::Complete
                })
        });
        if self.project_complete
            && self.default_count == 0
            && !self.source_files.is_empty()
            && all_block_definitions_complete
        {
            self.diagnostics.push(diagnostics::missing_default_block(
                first_validation_source_span(&self.source_files),
            ));
        }
    }
    pub(super) fn validate_source_file(&mut self, input: ValidationInput<'a>) {
        let source_file = input.source_file();
        self.participation = self
            .effective_participation
            .get(source_file.path.as_str())
            .copied()
            .unwrap_or_else(|| input.participation());
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
        if !self.project_complete {
            return;
        }
        let span = first_source_span(std::iter::once(source_file));
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
