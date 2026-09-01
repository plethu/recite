use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    Block, BlockReference, Choice, ChoiceEcho, DivertTarget, SourceFile, SourceSpan, Statement,
};

use super::participation::{ValidationCompleteness, ValidationInput, ValidationParticipation};
use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_block_id(&mut self, source_file: &'a SourceFile, block: &'a Block) {
        let key = (source_file.path.as_str(), block.id.as_str());
        if let Some(first_span) = self.block_ids.get(&key) {
            self.diagnostics.push(diagnostics::duplicate_block_id(
                &block.id,
                block.span.clone(),
                first_span.clone(),
            ));
        } else {
            self.block_ids.insert(key, block.span.clone());
        }

        if self.project_complete
            && let Some((first_file, first_span)) = self.compiled_block_ids.get(block.id.as_str())
        {
            if *first_file != source_file.path {
                self.diagnostics
                    .push(diagnostics::ambiguous_compiled_block_id(
                        &block.id,
                        block.span.clone(),
                        first_span.clone(),
                    ));
            }
        } else {
            self.compiled_block_ids.insert(
                block.id.as_str(),
                (source_file.path.as_str(), block.span.clone()),
            );
        }
    }
    pub(super) fn validate_default_block(&mut self, block: &'a Block) {
        if !block.is_default {
            return;
        }

        self.default_count += 1;
        if let Some(first) = self.first_default
            && (self.project_complete || first.span.file == block.span.file)
        {
            self.diagnostics
                .push(diagnostics::ambiguous_default_block(block, first));
        } else {
            self.first_default = Some(block);
        }
    }
    pub(super) fn validate_choice_echo(&mut self, choice: &'a Choice) {
        let ChoiceEcho::Line(line_id) = &choice.echo else {
            return;
        };

        if self.stable_ids_incomplete() || !self.project_complete {
            return;
        }
        if !self.line_ids.contains(line_id.as_str()) {
            self.diagnostics
                .push(diagnostics::unknown_choice_echo_line(choice, line_id));
        }
    }
    pub(crate) fn validate_reference(
        &mut self,
        source_file: &'a SourceFile,
        target: &'a DivertTarget,
        span: &SourceSpan,
    ) {
        let DivertTarget::Block(reference) = target else {
            return;
        };

        if matches!(
            self.resolve_block_reference(source_file, reference),
            BlockLookup::Missing
        ) {
            self.diagnostics.push(diagnostics::unknown_block_reference(
                reference,
                span.clone(),
            ));
        }
    }
    fn resolve_block_reference(
        &self,
        source_file: &'a SourceFile,
        reference: &'a BlockReference,
    ) -> BlockLookup {
        let file = reference
            .file
            .as_deref()
            .unwrap_or(source_file.path.as_str());
        if !self.project_complete && file != source_file.path.as_str() {
            return BlockLookup::Indeterminate;
        }
        let Some(participation) = self.effective_participation.get(file) else {
            return BlockLookup::Missing;
        };
        if participation.block_definitions() == ValidationCompleteness::Incomplete {
            return BlockLookup::Indeterminate;
        }
        let Some(blocks) = self.blocks.get(file) else {
            return BlockLookup::Missing;
        };

        if blocks.contains(reference.block_id.as_str()) {
            BlockLookup::Resolved
        } else {
            BlockLookup::Missing
        }
    }

    fn stable_ids_incomplete(&self) -> bool {
        self.effective_participation
            .values()
            .any(|participation| participation.stable_ids() == ValidationCompleteness::Incomplete)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockLookup {
    Resolved,
    Missing,
    Indeterminate,
}

pub(super) fn collect_line_ids<'a>(
    source_files: &[ValidationInput<'a>],
    effective_participation: &BTreeMap<&'a str, ValidationParticipation>,
) -> BTreeSet<&'a str> {
    let mut line_ids = BTreeSet::new();

    for source_file in source_files {
        let path = source_file.source_file().path.as_str();
        if effective_participation
            .get(path)
            .is_none_or(|participation| {
                participation.stable_ids() != ValidationCompleteness::Complete
            })
        {
            continue;
        }
        source_file
            .source_file()
            .visit_statements_depth_first(&mut |statement| {
                if let Statement::Line(line) = statement
                    && let Some(id) = &line.id
                {
                    line_ids.insert(id.as_str());
                }
            });
    }

    line_ids
}
