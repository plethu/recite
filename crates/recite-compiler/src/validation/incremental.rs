//! The authoring kernel separates local checks from project-index checks.
//! Batch validation still executes both through the same validation methods.
use super::{
    ValidationInput, ValidationReport, project::sort_diagnostics_by_source, state::Validator,
};
use recite_core::{ast::Statement, schema::ProjectSchema};

mod borrowed;
mod facts;
pub(crate) use facts::{FactRanges, ProjectFacts};
mod index;
pub(crate) use index::ProjectIndex;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ValidationPhase {
    Complete,
    Local,
    Project,
}

pub(crate) fn validate_local(
    input: ValidationInput<'_>,
    schema: Option<&ProjectSchema>,
) -> ValidationReport {
    let mut validator = Validator::for_phase([input], schema, true, ValidationPhase::Local);
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);
    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}

fn validate_context(
    facts: &[&ProjectFacts],
    complete: bool,
    stable_complete: bool,
) -> Vec<recite_core::Diagnostic> {
    let diagnostics = borrowed::validate(facts, complete, stable_complete);
    if cfg!(test) {
        VALIDATED_DOCUMENTS.with(|count| count.set(count.get() + facts.len()));
    }
    diagnostics
}

thread_local! {
    pub(crate) static VALIDATED_DOCUMENTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl<'a> Validator<'a> {
    pub(super) fn validate_project_statements(&mut self, input: ValidationInput<'a>) {
        let file = input.source_file();
        for block in &file.blocks {
            if self.participation.block_definitions().is_complete() {
                self.validate_block_id(file, block);
                self.validate_default_block(block);
            }
            for root in &block.statements {
                root.visit_depth_first(&mut |statement| match statement {
                    Statement::Line(line) if self.participation.stable_ids().is_complete() => {
                        self.validate_line_localisable_id(line);
                    }
                    Statement::Choice(choice) => {
                        if self.participation.stable_ids().is_complete() {
                            self.validate_choice_localisable_id(choice);
                            self.validate_choice_echo(choice);
                        }
                        if self.participation.block_references().is_complete()
                            && let Some(target) = &choice.target
                        {
                            self.validate_reference(file, &target.target, &target.span);
                        }
                    }
                    Statement::Divert(divert)
                        if self.participation.block_references().is_complete() =>
                    {
                        self.validate_reference(file, &divert.target, &divert.span);
                    }
                    _ => {}
                });
            }
        }
    }
}
