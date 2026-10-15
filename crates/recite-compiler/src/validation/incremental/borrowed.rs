//! Compact project facts use the same checks as batch AST validation.
use super::{ProjectFacts, ValidationPhase, facts::Identity};
use crate::validation::{ValidationInput, state::Validator};
use recite_core::{
    Diagnostic,
    ast::{ChoiceEcho, SourceFile},
};

pub(super) fn validate(
    facts: &[&ProjectFacts],
    complete: bool,
    stable_complete: bool,
) -> Vec<Diagnostic> {
    let mut ordered = facts.to_vec();
    ordered.sort_by_key(|facts| facts.path.as_str());
    let headers: Vec<_> = ordered.iter().map(|facts| facts.project_header()).collect();
    let inputs = headers
        .iter()
        .zip(&ordered)
        .map(|(header, facts)| ValidationInput::new(header, facts.participation));
    let mut validator = Validator::for_phase(inputs, None, complete, ValidationPhase::Project);
    validator.stable_ids_complete = stable_complete;
    prime_echo_targets(&mut validator, &ordered);
    for (header, facts) in headers.iter().zip(ordered) {
        validate_document(&mut validator, header, facts);
    }
    validator.diagnostics
}

fn prime_echo_targets<'a>(validator: &mut Validator<'a>, facts: &[&'a ProjectFacts]) {
    if validator.project_complete
        && validator.stable_ids_complete
        && facts.iter().any(|facts| {
            !facts.blocks.is_empty()
                && validator.effective_participation[facts.path.as_str()]
                    .stable_ids()
                    .is_complete()
                && facts.depends_on_stable_completeness()
        })
    {
        // The AST owner also accepts non-frozen line IDs as echo targets. Only
        // participation, not the collision-check gate, controls this membership.
        validator.line_ids = Some(
            facts
                .iter()
                .filter(|facts| {
                    !facts.blocks.is_empty()
                        && validator.effective_participation[facts.path.as_str()]
                            .stable_ids()
                            .is_complete()
                })
                .flat_map(|facts| {
                    facts
                        .passages
                        .iter()
                        .filter_map(|passage| match &passage.identity {
                            Identity::Line(Some(id)) => Some(id.as_str()),
                            _ => None,
                        })
                })
                .collect(),
        );
    }
}

fn validate_document<'a>(
    validator: &mut Validator<'a>,
    header: &'a SourceFile,
    facts: &'a ProjectFacts,
) {
    validator.participation = validator.effective_participation[header.path.as_str()];
    validator.validate_source_path(header);
    let Some(first) = header.blocks.first() else {
        return;
    };
    if validator.participation.block_definitions().is_complete() {
        validator.validate_block_id(header, first);
        validator.validate_default_block(first);
    }
    if validator.participation.stable_ids().is_complete() {
        for passage in facts.passages.iter() {
            match &passage.identity {
                Identity::Line(Some(id)) if passage.frozen => {
                    validator.validate_project_line_id(id, &passage.span);
                }
                Identity::Choice(id, echo) => {
                    if passage.frozen
                        && let Some(id) = id
                    {
                        validator.validate_project_choice_id(id, &passage.span);
                    }
                    if let ChoiceEcho::Line(id) = echo {
                        validator.validate_echo_line(id, &passage.span);
                    }
                }
                _ => {}
            }
        }
    }
    if validator.participation.block_references().is_complete() {
        for reference in &facts.references {
            validator.validate_reference(header, &reference.target, &reference.span);
        }
    }
    // The former flattened projection put every passage/reference in its
    // first block. Keep that order, including related diagnostic spans.
    if validator.participation.block_definitions().is_complete() {
        for block in header.blocks.iter().skip(1) {
            validator.validate_block_id(header, block);
            validator.validate_default_block(block);
        }
    }
}

#[cfg(test)]
mod tests;
