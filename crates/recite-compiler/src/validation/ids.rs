use std::collections::BTreeSet;

use recite_core::{
    Block, BlockReference, Choice, ChoiceEcho, DivertTarget, SourceFile, SourceSpan, Statement,
};

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

        if let Some((first_file, first_span)) = self.compiled_block_ids.get(block.id.as_str()) {
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
        if let Some(first) = self.first_default {
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

        if !self.line_ids.contains(line_id.as_str()) {
            self.diagnostics
                .push(diagnostics::unknown_choice_echo_line(choice, line_id));
        }
    }
    pub(super) fn validate_reference(
        &mut self,
        source_file: &'a SourceFile,
        target: &'a DivertTarget,
        span: &SourceSpan,
    ) {
        let DivertTarget::Block(reference) = target else {
            return;
        };

        if !self.contains_block_reference(source_file, reference) {
            self.diagnostics.push(diagnostics::unknown_block_reference(
                reference,
                span.clone(),
            ));
        }
    }
    pub(super) fn contains_block_reference(
        &self,
        source_file: &'a SourceFile,
        reference: &'a BlockReference,
    ) -> bool {
        let file = reference
            .file
            .as_deref()
            .unwrap_or(source_file.path.as_str());
        let Some(blocks) = self.blocks.get(file) else {
            return false;
        };

        blocks.contains(reference.block_id.as_str())
    }
}

pub(super) fn collect_line_ids<'a>(source_files: &[&'a SourceFile]) -> BTreeSet<&'a str> {
    let mut line_ids = BTreeSet::new();

    for source_file in source_files {
        source_file.visit_statements_depth_first(&mut |statement| {
            if let Statement::Line(line) = statement
                && let Some(id) = &line.id
            {
                line_ids.insert(id.as_str());
            }
        });
    }

    line_ids
}
