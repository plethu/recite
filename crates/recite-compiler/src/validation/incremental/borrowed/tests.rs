use super::*;
use crate::validation::{ValidationCompleteness, ValidationParticipation};
use recite_core::{
    SourceId,
    ast::{Block, Choice, Line, SourceFile, SourceText, Statement},
};

fn facts(path: &str, source: &str) -> ProjectFacts {
    let parsed = recite_parser::parse(path, source).lower_source_file();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    ProjectFacts::collect(ValidationInput::all_complete(&parsed.source_file))
}

fn legacy(facts: &[&ProjectFacts], complete: bool, stable: bool) -> Vec<Diagnostic> {
    let sources: Vec<_> = facts.iter().map(|facts| legacy_source(facts)).collect();
    let inputs = sources
        .iter()
        .zip(facts)
        .map(|(source, facts)| ValidationInput::new(source, facts.participation));
    let mut validator = Validator::for_phase(inputs, None, complete, ValidationPhase::Project);
    validator.stable_ids_complete = stable;
    for input in validator.source_files.clone() {
        validator.validate_source_file(input);
    }
    validator.diagnostics
}

#[test]
fn borrowed_facts_preserve_complete_ordered_diagnostics_and_related_presentations() {
    let first = facts(
        "z.recite",
        ":: same default\n> line@11111111111111111111\n  First.\n? reply@22222222222222222222 echo=line(33333333333333333333)\n  Reply.\n  -> missing\n:: same default\n-> END\n",
    );
    let second = facts(
        "a.recite",
        ":: same default\n? reply@11111111111111111111 echo=line(33333333333333333333)\n  Reply.\n  -> missing\n> line@33333333333333333333\n  Later.\n-> END\n",
    );
    let mut unfrozen = second.clone();
    let passages = std::sync::Arc::make_mut(&mut unfrozen.passages);
    passages[0].frozen = false;
    if let Identity::Choice(id, _) = &mut passages[0].identity {
        *id = None;
    }
    passages[1].frozen = false;
    let mut empty = first.clone();
    empty.blocks = std::sync::Arc::default();
    let original = [first, second, unfrozen, empty];
    for bits in 0..8 {
        let mut same_path = original[0].clone();
        let completeness = |bit| {
            if bits & bit == 0 {
                ValidationCompleteness::Complete
            } else {
                ValidationCompleteness::Incomplete
            }
        };
        same_path.participation = ValidationParticipation::all_complete()
            .with_block_definitions(completeness(1))
            .with_block_references(completeness(2))
            .with_stable_ids(completeness(4));
        for reverse in [false, true] {
            let mut inputs: Vec<_> = original.iter().chain([&same_path]).collect();
            if reverse {
                inputs.reverse();
            }
            for complete in [false, true] {
                for stable in [false, true] {
                    assert_eq!(
                        sorted(
                            validate(&inputs, complete, stable)
                                .iter()
                                .cloned()
                                .collect()
                        ),
                        sorted(legacy(&inputs, complete, stable)),
                        "participation {bits}, reverse {reverse}, complete {complete}, stable {stable}"
                    );
                }
            }
        }
    }
}

fn legacy_source(facts: &ProjectFacts) -> SourceFile {
    let mut blocks: Vec<_> = facts
        .blocks
        .iter()
        .map(|block| {
            Block::new(block.id.clone(), Vec::new(), block.span.clone()).with_default(block.default)
        })
        .collect();
    if let Some(first) = blocks.first_mut() {
        first.statements = facts
            .passages
            .iter()
            .map(|passage| {
                let text = SourceText::new(String::new(), passage.span.clone());
                match &passage.identity {
                    Identity::Line(id) => {
                        let mut line = Line::new(id.clone(), text, passage.span.clone());
                        if !passage.frozen {
                            line.source_id = SourceId::Missing;
                        }
                        Statement::Line(line)
                    }
                    Identity::Choice(id, echo) => {
                        let mut choice = Choice::new(id.clone(), text, passage.span.clone());
                        if !passage.frozen {
                            choice.source_id = SourceId::Missing;
                        }
                        choice.echo = echo.clone();
                        Statement::Choice(choice)
                    }
                }
            })
            .chain(facts.references.iter().cloned().map(Statement::Divert))
            .collect();
    }
    SourceFile::new(&facts.path, blocks)
}

fn sorted(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    crate::validation::project::sort_diagnostics_by_source(&mut diagnostics);
    diagnostics
}
