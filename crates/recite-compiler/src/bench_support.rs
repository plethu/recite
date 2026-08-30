//! Maintainer-only compiler phase probes for Criterion benchmarks.

use recite_core::{
    CompiledDialogue, Diagnostic, DivertTarget, ProjectSchema, SourceFile, Statement,
};

use crate::compile::CompileError;
use crate::validation::project::sort_diagnostics_by_source;
use crate::validation::state::Validator;
use crate::wire::serialize_messagepack;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilerPhaseProbe {
    pub checked_items: usize,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn resolve_block_references(source_files: &[SourceFile]) -> CompilerPhaseProbe {
    let mut validator = Validator::for_block_reference_probe(source_files);
    let mut checked_items = 0;
    for source_file in validator.source_files.clone() {
        source_file
            .source_file
            .visit_statements_depth_first(&mut |statement| match statement {
                Statement::Choice(choice) => {
                    if let Some(target) = &choice.target {
                        if matches!(target.target, DivertTarget::Block(_)) {
                            checked_items += 1;
                        }
                        validator.validate_reference(
                            source_file.source_file,
                            &target.target,
                            &target.span,
                        );
                    }
                }
                Statement::Divert(divert) => {
                    if matches!(divert.target, DivertTarget::Block(_)) {
                        checked_items += 1;
                    }
                    validator.validate_reference(
                        source_file.source_file,
                        &divert.target,
                        &divert.span,
                    );
                }
                _ => {}
            });
    }
    phase_probe(checked_items, validator.diagnostics)
}

#[must_use]
pub fn validate_localisable_id_uniqueness(source_files: &[SourceFile]) -> CompilerPhaseProbe {
    let mut validator = Validator::for_localisable_id_probe(source_files);
    let mut checked_items = 0;
    for source_file in validator.source_files.clone() {
        source_file
            .source_file
            .visit_statements_depth_first(&mut |statement| match statement {
                Statement::Line(line) => {
                    checked_items += 1;
                    validator.validate_line_localisable_id(line);
                }
                Statement::Choice(choice) => {
                    checked_items += 1;
                    validator.validate_choice_localisable_id(choice);
                }
                _ => {}
            });
    }
    phase_probe(checked_items, validator.diagnostics)
}

#[must_use]
pub fn validate_markup(source_files: &[SourceFile], schema: &ProjectSchema) -> CompilerPhaseProbe {
    let mut validator = Validator::for_markup_probe(source_files, schema);
    let mut checked_items = 0;
    for source_file in validator.source_files.clone() {
        source_file
            .source_file
            .visit_statements_depth_first(&mut |statement| match statement {
                Statement::Line(line) => {
                    checked_items += 1;
                    validator.validate_markup(&line.source_text);
                }
                Statement::Choice(choice) => {
                    checked_items += 1;
                    validator.validate_markup(&choice.source_text);
                }
                _ => {}
            });
    }
    phase_probe(checked_items, validator.diagnostics)
}

pub fn serialize_compiled_asset(dialogue: &CompiledDialogue) -> Result<Vec<u8>, CompileError> {
    serialize_messagepack(dialogue)
}

fn phase_probe(checked_items: usize, mut diagnostics: Vec<Diagnostic>) -> CompilerPhaseProbe {
    sort_diagnostics_by_source(&mut diagnostics);
    CompilerPhaseProbe {
        checked_items,
        diagnostics,
    }
}
