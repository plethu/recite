use recite_core::{
    AvailabilityReasonId,
    ast::{
        Argument, ChoiceAvailabilityReasonOverride, ChoiceAvailabilityRequirement, Comment,
        ConditionCall, ConditionExpression, Divert, Effect, EffectMode, IfBranch,
        InterpolationBinding, InterpolationType, MatchArm, MatchBranch, MatchPattern,
    },
};

use super::super::*;

#[test]
fn relocated_ast_reports_every_span_owner_at_its_original_position() {
    let mut source = nested_source();
    let initial = validate_source_files(&[source.clone()]);
    assert_codes(&initial, ["RECITE_VALIDATE040"]);
    assert_eq!(initial.diagnostics[0].span, at(15));

    source.path = "dialogue/relocated.recite".to_owned();
    let report = validate_source_files(&[source]);
    let expected = [
        (1, "block"),
        (2, "comment"),
        (3, "line"),
        (4, "line-source-text"),
        (5, "choice"),
        (6, "choice-source-text"),
        (7, "choice-availability-requirement"),
        (8, "condition-expression"),
        (9, "condition-call"),
        (10, "condition-function"),
        (11, "condition-argument"),
        (12, "condition-expression"),
        (13, "choice-availability-reason"),
        (14, "choice-availability-reason-id"),
        (15, "choice-availability-reason-arguments"),
        (16, "choice-target"),
        (17, "divert"),
        (18, "if-branch"),
        (19, "condition-expression"),
        (20, "condition-call"),
        (21, "match-branch"),
        (22, "condition-call"),
        (23, "match-arm"),
        (24, "effect"),
        (25, "effect-mode"),
        (26, "effect-function"),
        (27, "effect-call"),
        (28, "effect-argument"),
        (29, "plural-source-text"),
        (30, "metadata-entry"),
        (31, "metadata-key"),
        (32, "metadata-value"),
    ];
    assert_eq!(report.diagnostics.len(), expected.len() + 1);
    let span_diagnostics = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.as_str() == "RECITE_VALIDATE008")
        .collect::<Vec<_>>();
    assert_eq!(span_diagnostics.len(), expected.len());
    for (diagnostic, (line, owner)) in span_diagnostics.into_iter().zip(expected) {
        assert_eq!(diagnostic.span, at(line));
        super::assert_presentation(diagnostic, "diagnostic-validate-008-file", owner);
        assert_eq!(
            diagnostic.message,
            format!(
                "invalid source span for {}: span file does not match source file",
                owner.replace('-', " ")
            )
        );
    }
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() != "RECITE_VALIDATE008")
            .collect::<Vec<_>>(),
        initial.diagnostics.iter().collect::<Vec<_>>()
    );
}

fn at(line: u32) -> SourceSpan {
    span_range("dialogue/source.recite", line, 2, line, 7)
}

fn nested_source() -> SourceFile {
    let requirement = ConditionExpression::grouped(
        ConditionExpression::not(
            ConditionExpression::Call(
                ConditionCall::new("ready", vec![Argument::identifier("player")], at(9))
                    .with_source_spans(at(10), vec![at(11)]),
            ),
            at(12),
        ),
        at(8),
    );
    let choice = Choice::new(
        Some(ChoiceId::new("22222222222222222222").expect("valid choice ID")),
        SourceText::new("Ask.", at(6)),
        at(5),
    )
    .with_availability_requirement(ChoiceAvailabilityRequirement::new(requirement, at(7)))
    .with_availability_reason_override(
        ChoiceAvailabilityReasonOverride::new(
            AvailabilityReasonId::new("not_ready").expect("valid reason ID"),
            at(13),
            at(14),
        )
        .with_argument_span(at(15)),
    )
    .with_target(ChoiceTarget::new(DivertTarget::End, at(16)));
    let line = Line::new(
        Some(LineId::new("11111111111111111111").expect("valid line ID")),
        SourceText::new("One item.", at(4)),
        at(3),
    )
    .with_plural_source_text(SourceText::new("{count} items.", at(29)))
    .with_interpolation_bindings(vec![InterpolationBinding::new(
        "count",
        "remaining",
        InterpolationType::Integer,
    )])
    .with_statements(vec![Statement::Choice(choice)]);
    let effect = Effect::new(
        EffectMode::Immediate,
        "record",
        vec![Argument::identifier("player")],
        at(24),
    )
    .with_source_spans(at(25), at(27), at(26), vec![at(28)]);
    let branch = IfBranch::new(
        ConditionExpression::and(
            vec![ConditionExpression::call("ready", Vec::new(), at(20))],
            at(19),
        ),
        vec![Statement::Match(MatchBranch::new(
            ConditionCall::new("phase", Vec::new(), at(22)),
            vec![MatchArm::new(
                MatchPattern::Wildcard,
                vec![Statement::Effect(effect)],
                at(23),
            )],
            at(21),
        ))],
        at(18),
    )
    .with_else_statements(vec![Statement::Divert(Divert::new(
        DivertTarget::End,
        at(17),
    ))]);
    SourceFile::new(
        "dialogue/source.recite",
        vec![
            Block::new(
                BlockId::new("start").expect("valid block ID"),
                vec![
                    Statement::Comment(Comment::new("Source note", at(2))),
                    Statement::Line(line),
                    Statement::If(branch),
                ],
                at(1),
            )
            .with_default(true)
            .with_metadata(SourceMetadata::from_entries(vec![
                SourceMetadataEntry::new("mood", SourceMetadataScalar::Symbol("calm".to_owned()))
                    .with_source_span(at(30))
                    .with_key_value_spans(at(31), Some(at(32))),
            ])),
        ],
    )
}
