//! Exact inputs to project validation, excluding file-local prose/schema checks.
//! Borrowed facts are compared only for reparsed documents, then discarded.
use crate::ValidationInput;
use recite_core::{ChoiceEcho, DivertTarget, SourceFile, SourceId, SourceSpan, Statement};

pub(crate) fn same_project_inputs(before: ValidationInput<'_>, after: ValidationInput<'_>) -> bool {
    before.participation() == after.participation()
        && before.source_file().path == after.source_file().path
        && facts(before.source_file()) == facts(after.source_file())
}

#[derive(PartialEq)]
enum Fact<'a> {
    Block(&'a str, bool, &'a SourceSpan),
    Line(Option<&'a str>, bool, &'a SourceSpan),
    Choice(Option<&'a str>, bool, &'a SourceSpan, &'a ChoiceEcho),
    Reference(&'a DivertTarget, &'a SourceSpan),
}

fn facts(file: &SourceFile) -> Vec<Fact<'_>> {
    let mut facts = Vec::new();
    for block in &file.blocks {
        facts.push(Fact::Block(
            block.id.as_str(),
            block.is_default,
            &block.span,
        ));
        for root in &block.statements {
            root.visit_depth_first(&mut |statement| match statement {
                Statement::Line(line) => facts.push(Fact::Line(
                    line.id.as_ref().map(|id| id.as_str()),
                    matches!(line.source_id, SourceId::Frozen { .. }),
                    &line.span,
                )),
                Statement::Choice(choice) => {
                    facts.push(Fact::Choice(
                        choice.id.as_ref().map(|id| id.as_str()),
                        matches!(choice.source_id, SourceId::Frozen { .. }),
                        &choice.span,
                        &choice.echo,
                    ));
                    if let Some(target) = &choice.target {
                        facts.push(Fact::Reference(&target.target, &target.span));
                    }
                }
                Statement::Divert(divert) => {
                    facts.push(Fact::Reference(&divert.target, &divert.span))
                }
                _ => {}
            });
        }
    }
    facts
}
